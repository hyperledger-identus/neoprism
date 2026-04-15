use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::TxId;
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::proto::MessageExt;
use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
use reqwest::Client;
use serde_json::json;

use crate::DltSink;
use crate::dlt::cardano_wallet::models::{Payment, PaymentAmount, TxRequest, TxResponse};

mod models {
    use identus_did_prism::dlt::TxId;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct TxResponse {
        pub id: TxId,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct TxRequest {
        pub passphrase: String,
        pub payments: Vec<Payment>,
        pub metadata: serde_json::Value,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Payment {
        pub address: String,
        pub amount: PaymentAmount,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct PaymentAmount {
        pub quantity: u64,
        pub unit: String,
    }
}

pub struct CardanoWalletSink {
    base_url: String,
    wallet_id: String,
    passphrase: String,
    payment_address: String,
    client: Client,
}

impl CardanoWalletSink {
    pub fn new(base_url: String, wallet_id: String, passphrase: String, payment_address: String) -> Self {
        Self {
            base_url,
            wallet_id,
            passphrase,
            payment_address,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl DltSink for CardanoWalletSink {
    async fn publish_operations(&self, operations: Vec<SignedPrismOperation>) -> Result<TxId, String> {
        let prism_object = PrismObject {
            block_content: Some(PrismBlock {
                operations,
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        };

        let metadata = encode_metadata(prism_object);
        let tx_request = TxRequest {
            metadata,
            passphrase: self.passphrase.clone(),
            payments: vec![Payment {
                address: self.payment_address.to_string(),
                amount: PaymentAmount {
                    quantity: 1_000_000,
                    unit: "lovelace".to_string(),
                },
            }],
        };

        let resp = self
            .client
            .post(format!("{}/wallets/{}/transactions", self.base_url, self.wallet_id))
            .json(&tx_request)
            .send()
            .await
            .map_err(|e| format!("unable to submit transaction: {e}"))?;

        if resp.status().is_success() {
            let tx_resp = resp
                .json::<TxResponse>()
                .await
                .map_err(|e| format!("unable to decode transaction submission response: {e}"))?;
            Ok(tx_resp.id)
        } else {
            Err(format!(
                "cardano wallet did not return success status (status: {}, body: {:?})",
                resp.status().as_u16(),
                resp.text().await
            ))
        }
    }
}

fn encode_metadata(prism_object: PrismObject) -> serde_json::Value {
    let bytes = prism_object.encode_to_vec();
    let byte_group = bytes
        .chunks(64)
        .map(|b| HexStr::from(b).to_string())
        .map(|hex_str| json!({"bytes": hex_str}))
        .collect::<Vec<_>>();

    json!({
        "21325": {
            "map": [
                { "k" : { "string": "v" }, "v" : { "int" : 1 } },
                {
                    "k" : { "string" : "c" },
                    "v" : { "list" : byte_group }
                }
            ]
        }
    })
}
