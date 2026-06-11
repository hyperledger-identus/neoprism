use std::str::FromStr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use identus_did_core::{
    Did, DidDocument, DidDocumentMetadata, DidResolutionError, DidResolutionErrorCode, DidResolutionMetadata,
    DidResolver, ResolutionResult,
};
use identus_did_resolver_http::{DidResolverStateDyn, did_resolver_http_binding};
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Mock resolver
// ---------------------------------------------------------------------------

/// A mock resolver that returns a preset result for any DID.
#[derive(Clone)]
struct MockResolver {
    result_fn: Arc<dyn Fn(&Did) -> ResolutionResult + Send + Sync>,
}

impl MockResolver {
    fn new(f: impl Fn(&Did) -> ResolutionResult + Send + Sync + 'static) -> Self {
        Self { result_fn: Arc::new(f) }
    }

    fn success() -> Self {
        Self::new(|_| {
            ResolutionResult::success(DidDocument {
                context: vec!["https://www.w3.org/ns/did/v1".to_string()],
                id: Did::from_str("did:example:123").unwrap(),
                also_known_as: None,
                verification_method: vec![],
                authentication: None,
                assertion_method: None,
                key_agreement: None,
                capability_invocation: None,
                capability_delegation: None,
                service: None,
            })
        })
    }

    /// Echoes the DID received by the resolver into the document `id`, so tests
    /// can verify the handler decodes the path segment and forwards the right DID.
    fn echo() -> Self {
        Self::new(|did| {
            ResolutionResult::success(DidDocument {
                context: vec!["https://www.w3.org/ns/did/v1".to_string()],
                id: did.clone(),
                also_known_as: None,
                verification_method: vec![],
                authentication: None,
                assertion_method: None,
                key_agreement: None,
                capability_invocation: None,
                capability_delegation: None,
                service: None,
            })
        })
    }

    fn not_found() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::NotFound,
                    title: Some("Not Found".to_string()),
                    detail: Some("DID not found".to_string()),
                }),
            },
            did_document_metadata: Default::default(),
        })
    }

    fn deactivated() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: Default::default(),
            did_document_metadata: DidDocumentMetadata {
                deactivated: Some(true),
                ..Default::default()
            },
        })
    }

    fn method_not_supported() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::MethodNotSupported,
                    title: Some("Method Not Supported".to_string()),
                    detail: Some("DID method not supported".to_string()),
                }),
            },
            did_document_metadata: Default::default(),
        })
    }

    fn internal_error() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::InternalError,
                    title: Some("Internal Error".to_string()),
                    detail: Some("something went wrong".to_string()),
                }),
            },
            did_document_metadata: Default::default(),
        })
    }

    fn invalid_did_url() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::InvalidDidUrl,
                    title: Some("Invalid DID URL".to_string()),
                    detail: None,
                }),
            },
            did_document_metadata: Default::default(),
        })
    }

    fn invalid_options() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::InvalidOptions,
                    title: Some("Invalid Options".to_string()),
                    detail: None,
                }),
            },
            did_document_metadata: Default::default(),
        })
    }

    fn representation_not_supported() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::RepresentationNotSupported,
                    title: Some("Not Acceptable".to_string()),
                    detail: None,
                }),
            },
            did_document_metadata: Default::default(),
        })
    }
    fn unsupported_public_key_type() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::UnsupportedPublicKeyType,
                    title: None,
                    detail: None,
                }),
            },
            did_document_metadata: Default::default(),
        })
    }

    fn invalid_public_key() -> Self {
        Self::new(|_| ResolutionResult {
            did_document: None,
            did_resolution_metadata: DidResolutionMetadata {
                content_type: None,
                error: Some(DidResolutionError {
                    r#type: DidResolutionErrorCode::InvalidPublicKey,
                    title: None,
                    detail: None,
                }),
            },
            did_document_metadata: Default::default(),
        })
    }
}

#[async_trait::async_trait]
impl DidResolver for MockResolver {
    async fn resolve(&self, did: &Did, _options: &identus_did_core::ResolutionOptions) -> ResolutionResult {
        (self.result_fn)(did)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_app(resolver: MockResolver) -> axum::Router {
    let binding = did_resolver_http_binding("/did/{did}", Default::default());
    let state = DidResolverStateDyn {
        resolver: Arc::new(resolver),
    };
    binding.router.with_state(state)
}

async fn send_request(app: axum::Router, did: &str, accept: Option<&str>) -> (StatusCode, String, String) {
    // Percent-encode colons and other special chars in the DID for use in URI path
    let encoded_did = did.replace(':', "%3A");
    let path = format!("/did/{encoded_did}");
    send_request_raw(app, &path, accept).await
}

async fn send_request_raw(app: axum::Router, path: &str, accept: Option<&str>) -> (StatusCode, String, String) {
    let mut builder = Request::builder().uri(path).method("GET");
    if let Some(accept) = accept {
        builder = builder.header(header::ACCEPT, accept);
    }
    let request = builder.body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body).to_string();
    (status, content_type, body_str)
}

// ---------------------------------------------------------------------------
// Tests: successful resolution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_success_default_accept_returns_application_did() {
    let app = make_app(MockResolver::success());
    let (status, content_type, body) = send_request(app, "did:example:123", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/did");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // Body should be the DID document (not the full resolution result)
    assert!(json.get("@context").is_some());
    assert_eq!(json["id"], "did:example:123");
}

#[tokio::test]
async fn resolve_success_accept_application_json() {
    let app = make_app(MockResolver::success());
    let (status, content_type, body) = send_request(app, "did:example:123", Some("application/json")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/json");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // application/json returns just the did_document without content-type header suffix
    assert!(json.get("@context").is_some());
}

#[tokio::test]
async fn resolve_success_accept_application_did() {
    let app = make_app(MockResolver::success());
    let (status, content_type, body) = send_request(app, "did:example:123", Some("application/did")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/did");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.get("@context").is_some());
}

#[tokio::test]
async fn resolve_success_accept_application_did_resolution() {
    let app = make_app(MockResolver::success());
    let (status, content_type, body) = send_request(app, "did:example:123", Some("application/did-resolution")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // Full resolution result includes didResolutionMetadata
    assert!(json.get("didResolutionMetadata").is_some());
    assert!(json.get("didDocument").is_some());
}

#[tokio::test]
async fn resolve_success_accept_multiple_includes_json() {
    // NOTE: There is a known bug where the Accept header values are not trimmed
    // after splitting on comma. So "application/json" with a leading space won't
    // match. This test uses no-space commas to verify the intended behavior works.
    let app = make_app(MockResolver::success());
    let (status, content_type, _body) =
        send_request(app, "did:example:123", Some("text/html,application/json,*/*")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/json");
}

#[tokio::test]
async fn resolve_passes_decoded_did_from_path_to_resolver() {
    let app = make_app(MockResolver::echo());
    let requested_did = "did:prism:9bf36a6dd4090ad66e359a0c041e25662c3f84c00467e9a61eeba68477c8a595";
    let (status, _content_type, body) = send_request(app, requested_did, None).await;

    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // The percent-encoded path segment must be decoded back to the original DID
    assert_eq!(json["id"], requested_did);
}

// ---------------------------------------------------------------------------
// Tests: invalid DID
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_invalid_did_returns_bad_request() {
    let app = make_app(MockResolver::success());
    // "not-a-did" is not a valid DID (no method)
    let (status, content_type, body) = send_request(app, "not-a-did", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#INVALID_DID");
}

#[tokio::test]
async fn resolve_invalid_did_with_fragment_returns_bad_request() {
    let app = make_app(MockResolver::success());
    // URL-encode the fragment so it's part of the path, not the URL fragment
    let (status, content_type, body) = send_request_raw(app, "/did/did:example:123%23key-1", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#INVALID_DID");
}

#[tokio::test]
async fn resolve_invalid_did_with_query_returns_bad_request() {
    let app = make_app(MockResolver::success());
    // URL-encode the ? so it's part of the path, not the query string
    let (status, content_type, body) = send_request_raw(app, "/did/did:example:123%3Fservice=abc", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#INVALID_DID");
}

// ---------------------------------------------------------------------------
// Tests: not found
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_not_found_returns_404() {
    let app = make_app(MockResolver::not_found());
    let (status, content_type, body) = send_request(app, "did:example:456", None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#NOT_FOUND");
}

// ---------------------------------------------------------------------------
// Tests: deactivated
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_deactivated_returns_gone() {
    let app = make_app(MockResolver::deactivated());
    let (status, content_type, body) = send_request(app, "did:example:789", None).await;

    assert_eq!(status, StatusCode::GONE);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["didDocumentMetadata"]["deactivated"], true);
}

// ---------------------------------------------------------------------------
// Tests: method not supported
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_method_not_supported_returns_not_implemented() {
    let app = make_app(MockResolver::method_not_supported());
    let (status, content_type, body) = send_request(app, "did:example:abc", None).await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#METHOD_NOT_SUPPORTED");
}

// ---------------------------------------------------------------------------
// Tests: internal error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_internal_error_returns_500() {
    let app = make_app(MockResolver::internal_error());
    let (status, content_type, body) = send_request(app, "did:example:err", None).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#INTERNAL_ERROR");
}

// ---------------------------------------------------------------------------
// Tests: other error codes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_invalid_did_url_returns_bad_request() {
    let app = make_app(MockResolver::invalid_did_url());
    let (status, content_type, body) = send_request(app, "did:example:url", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#INVALID_DID_URL");
}

#[tokio::test]
async fn resolve_invalid_options_returns_bad_request() {
    let app = make_app(MockResolver::invalid_options());
    let (status, content_type, body) = send_request(app, "did:example:opt", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#INVALID_OPTIONS");
}

#[tokio::test]
async fn resolve_representation_not_supported_returns_not_acceptable() {
    let app = make_app(MockResolver::representation_not_supported());
    let (status, content_type, body) = send_request(app, "did:example:rep", None).await;

    assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://www.w3.org/ns/did#REPRESENTATION_NOT_SUPPORTED");
}

// ---------------------------------------------------------------------------
// Tests: Accept header content negotiation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn accept_application_did_resolution_with_not_found() {
    let app = make_app(MockResolver::not_found());
    let (status, content_type, body) =
        send_request(app, "did:example:missing", Some("application/did-resolution")).await;

    // Error results always use application/did-resolution content type
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.get("didResolutionMetadata").is_some());
}

#[tokio::test]
async fn accept_wildcard_returns_application_did() {
    let app = make_app(MockResolver::success());
    let (status, content_type, _body) = send_request(app, "did:example:123", Some("*/*")).await;

    assert_eq!(status, StatusCode::OK);
    // Wildcard doesn't match any specific type, falls through to default (application/did)
    assert_eq!(content_type, "application/did");
}

#[tokio::test]
async fn no_accept_header_with_deactivated_returns_gone_with_did_resolution() {
    let app = make_app(MockResolver::deactivated());
    let (status, content_type, body) = send_request(app, "did:example:deact", None).await;

    // Deactivated DID should return GONE regardless of Accept header
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["didDocumentMetadata"]["deactivated"], true);
}

// ---------------------------------------------------------------------------
// Tests: binding creation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn binding_creates_functional_router() {
    // Verify the binding creates a router that can handle requests
    let binding = did_resolver_http_binding("/resolve/{did}", Default::default());
    let state = DidResolverStateDyn {
        resolver: Arc::new(MockResolver::success()),
    };
    let app = binding.router.with_state(state);

    let request = Request::builder()
        .uri("/resolve/did:example:test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Tests: error response body format
// ---------------------------------------------------------------------------

#[tokio::test]
async fn error_response_includes_title_and_detail() {
    let app = make_app(MockResolver::not_found());
    let (_status, _content_type, body) = send_request(app, "did:example:detail", None).await;

    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["title"], "Not Found");
    assert_eq!(error["detail"], "DID not found");
}

#[tokio::test]
async fn deactivated_result_has_null_did_document() {
    let app = make_app(MockResolver::deactivated());
    let (_status, _content_type, body) = send_request(app, "did:example:deact2", None).await;

    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["didDocument"].is_null());
}

// ---------------------------------------------------------------------------
// Tests: additional error code coverage
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_unsupported_public_key_type_returns_not_implemented() {
    let app = make_app(MockResolver::unsupported_public_key_type());
    let (status, content_type, body) = send_request(app, "did:example:unsupported", None).await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(content_type, "application/did-resolution");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://w3id.org/security#UNSUPPORTED_PUBLIC_KEY_TYPE");
}

#[tokio::test]
async fn resolve_invalid_public_key_returns_internal_error_via_catch_all() {
    // InvalidPublicKey falls through to the catch-all branch (Some(_) => INTERNAL_SERVER_ERROR)
    let app = make_app(MockResolver::invalid_public_key());
    let (status, _content_type, body) = send_request(app, "did:example:pkerr", None).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let error = &json["didResolutionMetadata"]["error"];
    assert_eq!(error["type"], "https://w3id.org/security#INVALID_PUBLIC_KEY");
}
