use identus_apollo::jwk::EncodeJwk;
use identus_did_prism::did::operation::{KeyUsage, PublicKey, PublicKeyData};
use maud::{Markup, html};

use super::identity::copy;
use crate::http::components::disclosure_icon;

pub(super) fn public_keys(keys: &[PublicKey]) -> Markup {
    let mut sorted = keys.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|key| key.id.to_string());
    html! {
        section id="public-keys" class="prism-panel prism-keys" aria-labelledby="keys-title" {
            div class="prism-section-heading" {
                h2 id="keys-title" { "Public keys" }
                span class="prism-count" { (keys.len()) " keys" }
            }
            @if keys.is_empty() {
                p class="prism-muted" { "No public keys" }
            } @else {
                @for key in sorted {
                    (key_disclosure(key))
                }
                p class="prism-key-help prism-muted" { "Select a key for its full usage, curve and JWK coordinates." }
            }
        }
    }
}

fn key_disclosure(key: &PublicKey) -> Markup {
    let jwk = match &key.data {
        PublicKeyData::Master { data } | PublicKeyData::Vdr { data } => data.encode_jwk(),
        PublicKeyData::Other { data, .. } => data.encode_jwk(),
    };
    let id = key.id.to_string();
    let usage = key.data.usage();
    let friendly_usage = match usage {
        KeyUsage::MasterKey => "Master key",
        KeyUsage::IssuingKey => "Issuing key",
        KeyUsage::KeyAgreementKey => "Key agreement key",
        KeyUsage::AuthenticationKey => "Authentication key",
        KeyUsage::RevocationKey => "Revocation key",
        KeyUsage::CapabilityInvocationKey => "Capability invocation key",
        KeyUsage::CapabilityDelegationKey => "Capability delegation key",
        KeyUsage::VdrKey => "VDR key",
    };
    html! {
        details class="prism-key" {
            summary {
                span class="prism-key-label" {
                    code { (id) }
                    span class="prism-muted" { (friendly_usage) " · " (jwk.crv) }
                }
                (disclosure_icon())
            }
            dl class="prism-key-fields" {
                (field("ID", Some(&id), &format!("key {id} ID")))
                (field("Usage", Some(&format!("{usage:?}")), &format!("key {id} usage")))
                (field("Curve", Some(&jwk.crv), &format!("key {id} curve")))
                (field("JWK kty", Some(&jwk.kty), &format!("key {id} JWK type")))
                (field("JWK X", jwk.x.as_ref().map(ToString::to_string).as_deref(), &format!("key {id} X coordinate")))
                (field("JWK Y", jwk.y.as_ref().map(ToString::to_string).as_deref(), &format!("key {id} Y coordinate")))
            }
        }
    }
}

fn field(label: &str, value: Option<&str>, copy_label: &str) -> Markup {
    html! {
        div class="prism-data-field" {
            dt { (label) }
            dd {
                @match value {
                    Some(value) => {
                        code { (value) }
                        (copy(copy_label, value))
                    }
                    None => span class="prism-muted" { "Not applicable" },
                }
            }
        }
    }
}
