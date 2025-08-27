use graphql_client::{GraphQLQuery, Response};
use identus_apollo::hex::HexStr;
use identus_did_midnight::did::MidnightDid;
use identus_did_midnight::dlt::ContractState;

type HexEncoded = HexStr;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum IndexerApiError {
    #[display("http error when calling {url}: {source}")]
    HttpError { source: reqwest::Error, url: String },
    #[display("json error when parsing response from {url}: {source}")]
    JsonError { source: reqwest::Error, url: String },
    #[display("graphql error from {url}: {messages:?}")]
    GraphqlError { messages: Vec<String>, url: String },
    #[display("indexer response from {url} for contract address {address} has no data")]
    NoData { url: String, address: String },
    #[display("contract action for address {address} not found in indexer response from {url}")]
    NoContractAction { url: String, address: String },
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.gql",
    query_path = "graphql/query.gql",
    response_derives = "Debug"
)]
struct ContractStateQuery;

pub async fn get_contract_state(url: &str, did: &MidnightDid) -> Result<ContractState, IndexerApiError> {
    let address_bytes = {
        let mut global_addr = [0u8; 35];
        let network_addr = did.contract_address.as_slice();
        global_addr[2..].copy_from_slice(network_addr);
        global_addr
    };
    let address = HexStr::from(address_bytes).to_string();
    let variables = contract_state_query::Variables {
        address: Some(address.clone()),
    };
    let request_body = ContractStateQuery::build_query(variables);
    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| IndexerApiError::HttpError {
            source: e,
            url: url.to_string(),
        })?;
    let response_body: Response<contract_state_query::ResponseData> = res
        .json::<Response<contract_state_query::ResponseData>>()
        .await
        .map_err(|e| IndexerApiError::JsonError {
            source: e,
            url: url.to_string(),
        })?;
    tracing::info!("indexer response: {:#?}", response_body);

    if let Some(errors) = response_body.errors {
        if !errors.is_empty() {
            Err(IndexerApiError::GraphqlError {
                messages: errors.into_iter().map(|i| i.to_string()).collect(),
                url: url.to_string(),
            })?
        }
    }
    let data = response_body.data.ok_or(IndexerApiError::NoData {
        url: url.to_string(),
        address: address.clone(),
    })?;
    let contract = data.contract_action.ok_or(IndexerApiError::NoContractAction {
        url: url.to_string(),
        address: address,
    })?;
    Ok(contract.state.into())
}
