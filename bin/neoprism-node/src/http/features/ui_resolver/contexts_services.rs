use identus_did_prism::did::operation::{Service, ServiceEndpoint, ServiceEndpointValue, ServiceType};
use maud::{Markup, html};
use serde_json::{Value, json};

use super::identity::copy;
use crate::http::components::disclosure_icon;

pub(super) fn contexts(contexts: &[String]) -> Markup {
    html! {
        section id="contexts" tabindex="-1" class="prism-panel prism-contexts" aria-labelledby="contexts-title" {
            @if contexts.is_empty() {
                h2 id="contexts-title" { "Contexts · 0" }
                p class="prism-muted" { "No contexts" }
            } @else {
                details class="prism-context-disclosure" {
                    summary {
                        h2 id="contexts-title" { "Contexts · " (contexts.len()) }
                        (disclosure_icon())
                    }
                    p class="prism-muted" { "Additional @context values in their supplied order." }
                    ol class="prism-context-list" {
                        @for (index, context) in contexts.iter().enumerate() {
                            li {
                                div class="prism-context-value" {
                                    code { (context) }
                                    (copy(&format!("context {}", index + 1), context))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn services(services: &[Service]) -> Markup {
    html! {
        section id="services" tabindex="-1" class="prism-panel prism-services" aria-labelledby="services-title" {
            h2 id="services-title" { "Services · " (services.len()) }
            @if services.is_empty() {
                p class="prism-muted" { "No services" }
            } @else {
                @for service in services {
                    (service_disclosure(service))
                }
            }
        }
    }
}

fn service_disclosure(service: &Service) -> Markup {
    let id = service.id.to_string();
    let (types, type_shape) = match &service.r#type {
        ServiceType::One(value) => (json!(value.to_string()), "Single type".to_owned()),
        ServiceType::Many(values) => (
            json!(values.iter().map(ToString::to_string).collect::<Vec<_>>()),
            format!("Multiple types · {}", values.len()),
        ),
    };
    let (endpoint, endpoint_shape) = match &service.service_endpoint {
        ServiceEndpoint::One(value) => (
            endpoint_value(value),
            match value {
                ServiceEndpointValue::Uri(_) => "URI endpoint".to_owned(),
                ServiceEndpointValue::Json(_) => "Object endpoint".to_owned(),
            },
        ),
        ServiceEndpoint::Many(values) => (
            Value::Array(values.iter().map(endpoint_value).collect()),
            format!("Array endpoints · {}", values.len()),
        ),
    };
    let readable = pretty(&json!({"id": id, "type": types, "serviceEndpoint": endpoint}));
    // Preserve the exact captured Debug text; readable JSON is a separate parsed-state projection.
    let raw = format!("{service:?}");
    html! {
        details class="prism-service" aria-label=(format!("Service {id}")) {
            summary {
                span class="prism-service-label" {
                    span { "Service " code { (id) } }
                    span class="prism-muted" { (type_shape) " · " (endpoint_shape) }
                }
                (disclosure_icon())
            }
            dl class="prism-service-fields" {
                (field("ID", &id, &format!("service {id} ID")))
                (field(&type_shape, &match &types { Value::String(value) => value.clone(), _ => pretty(&types) }, &format!("service {id} types")))
                (field(&endpoint_shape, &match &endpoint { Value::String(value) => value.clone(), _ => pretty(&endpoint) }, &format!("service {id} endpoint")))
            }
            details class="prism-service-representation" {
                summary { "Readable service JSON" (disclosure_icon()) }
                p class="prism-muted" { "Serialized from the resolved typed state; not the authored wire formatting." }
                div class="prism-service-code" {
                    pre { code { (readable) } }
                    (copy(&format!("service {id} readable JSON"), &readable))
                }
            }
            details class="prism-service-representation" {
                summary { "Captured service representation" (disclosure_icon()) }
                p class="prism-muted" {
                    "Exact captured resolved fields are retained below. Updates may normalize these fields. Authored wire formatting is not available; readable JSON does not reconstruct it."
                }
                dl class="prism-service-fields" {
                    (field("Captured ID", &service.orig.id, &format!("service {id} captured ID")))
                    (field("Captured Type", &service.orig.type_, &format!("service {id} captured types")))
                    (field("Captured Endpoint", &service.orig.service_endpoint, &format!("service {id} captured endpoint")))
                }
                details class="prism-service-representation" {
                    summary { "Raw typed representation" (disclosure_icon()) }
                    div class="prism-service-code" {
                        pre { code { (raw) } }
                        (copy(&format!("service {id} raw typed representation"), &raw))
                    }
                }
            }
        }
    }
}

fn endpoint_value(endpoint: &ServiceEndpointValue) -> Value {
    match endpoint {
        ServiceEndpointValue::Uri(uri) => json!(uri),
        ServiceEndpointValue::Json(object) => json!(object),
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).expect("JSON values can be serialized")
}

fn field(label: &str, value: &str, copy_label: &str) -> Markup {
    html! {
        div class="prism-data-field" {
            dt { (label) }
            dd { pre { code { (value) } } (copy(copy_label, value)) }
        }
    }
}
