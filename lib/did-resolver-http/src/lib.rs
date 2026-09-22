//! NeoPRISM compatibility facade for the sdk-rust DID Resolution HTTP binding.
//!
//! The generic HTTP behavior is owned by sdk-rust. This crate retains only the
//! historical NeoPRISM binding shape, dynamic resolver state, mount-path
//! composition, and OpenAPI tag/path customization.

use std::sync::Arc;

use axum::Router;
use identus_did_core::DidResolver;
use identus_did_core::sdk_adapter::SdkDidResolverAdapter;

const SDK_RESOLVER_PATH: &str = "/{did}";

/// A NeoPRISM-compatible binding whose transport behavior is supplied by sdk-rust.
pub struct DidResolverHttpBinding {
    pub router: DidResolverRouter,
    #[cfg(feature = "openapi")]
    pub openapi: utoipa::openapi::OpenApi,
}

/// Dynamic NeoPRISM resolver state accepted by the historical composition API.
#[derive(Clone)]
pub struct DidResolverStateDyn {
    pub resolver: Arc<dyn DidResolver + Send + Sync>,
}

/// NeoPRISM-specific presentation options retained around the SDK binding.
#[derive(Default)]
pub struct HttpBindingOptions {
    pub openapi_tags: Option<Vec<String>>,
}

/// Deferred router composition preserving NeoPRISM's `with_state` call site.
#[derive(Clone, Debug)]
pub struct DidResolverRouter {
    path: String,
}

impl Default for DidResolverRouter {
    fn default() -> Self {
        Self {
            path: SDK_RESOLVER_PATH.to_owned(),
        }
    }
}

impl DidResolverRouter {
    /// Bind one NeoPRISM resolver and return a state-closed Axum router.
    pub fn with_state(self, state: DidResolverStateDyn) -> Router {
        let resolver = Arc::new(SdkDidResolverAdapter::new(state.resolver));
        let sdk_router = identus_sdk_did_resolver_http::did_resolver_http_router(resolver);
        let prefix = self
            .path
            .strip_suffix(SDK_RESOLVER_PATH)
            .expect("DID resolver path must end with `/{did}`");
        if prefix.is_empty() {
            sdk_router
        } else {
            Router::new().nest(prefix, sdk_router)
        }
    }
}

/// Build the historical NeoPRISM facade around the sdk-rust HTTP adapter.
#[must_use]
pub fn did_resolver_http_binding(path: &str, options: HttpBindingOptions) -> DidResolverHttpBinding {
    assert!(
        path.ends_with(SDK_RESOLVER_PATH),
        "DID resolver path must end with `/{{did}}`"
    );

    #[cfg(not(feature = "openapi"))]
    let _ = options;

    #[cfg(feature = "openapi")]
    let openapi = {
        let mut openapi = identus_sdk_did_resolver_http::did_resolver_http_openapi();
        if let Some(mut path_item) = openapi.paths.get_path_item(SDK_RESOLVER_PATH).cloned() {
            if let Some(operation) = path_item.get.as_mut() {
                operation.tags = options.openapi_tags;
            }
            openapi.paths.paths.insert(path.to_owned(), path_item);
            openapi.paths.paths.remove(SDK_RESOLVER_PATH);
        }
        openapi
    };

    DidResolverHttpBinding {
        router: DidResolverRouter { path: path.to_owned() },
        #[cfg(feature = "openapi")]
        openapi,
    }
}
