use lazybe::macros::typed_uri;

use crate::http::ui_explorer::models::PageQuery;
use crate::http::ui_resolver::models::DidQuery;

// assets
typed_uri!(AssetBase, "assets");
typed_uri!(AssetStyleSheet, "assets" / "styles.css");

// misc
typed_uri!(Home, "");
typed_uri!(Swagger, "swagger-ui");

// UI resolver
typed_uri!(Resolver, "resolver" ? Option<DidQuery>);

// UI explorer
typed_uri!(Explorer, "explorer" ? Option<PageQuery>);
typed_uri!(ExplorerDltCursor, "explorer" / "dlt-cursor");
typed_uri!(ExplorerDidList, "explorer" / "did-list" ? Option<PageQuery>);

// API system
typed_uri!(ApiHealth, "api" / "_system" / "health");
typed_uri!(ApiAppMeta, "api" / "_system" / "metadata");

// API submitter
typed_uri!(ApiSignedOpSubmissions, "api" / "signed-operation-submissions");

// API indexer
typed_uri!(ApiDid, "api" / "dids" / (did: String));
typed_uri!(ApiDidData, "api" / "did-data" / (did: String));
typed_uri!(ApiIndexerStats, "api" / "indexer-stats");
typed_uri!(ApiVdrBlob, "api" / "vdr-data" / (entry_hash: String));
