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
    #[display("missing data fields {fields:?} for contract address {address} in indexer response from {url}")]
    MissingDataFields {
        url: String,
        address: HexStr,
        fields: Vec<&'static str>,
    },
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.gql",
    query_path = "graphql/query.gql",
    response_derives = "Debug"
)]
struct ContractStateQuery;

pub async fn get_contract_state(url: &str, did: &MidnightDid) -> Result<ContractState, IndexerApiError> {
    let address_bytes = did.global_contract_address();
    let address = HexStr::from(address_bytes);
    let variables = contract_state_query::Variables {
        address: Some(address.clone()),
    };
    let request_body = ContractStateQuery::build_query(variables);
    let response_body = execute_graphql_query::<contract_state_query::ResponseData>(url, &request_body).await?;
    let data = response_body.data.ok_or(IndexerApiError::MissingDataFields {
        url: url.to_string(),
        address: address.clone(),
        fields: vec!["data"],
    })?;
    let contract = data.contract_action.ok_or(IndexerApiError::MissingDataFields {
        url: url.to_string(),
        address,
        fields: vec!["contract_action"],
    })?;
    Ok(contract.state.into())
}

async fn execute_graphql_query<T: serde::de::DeserializeOwned>(
    url: &str,
    request_body: &impl serde::Serialize,
) -> Result<Response<T>, IndexerApiError> {
    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .json(request_body)
        .send()
        .await
        .map_err(|e| IndexerApiError::HttpError {
            source: e,
            url: url.to_string(),
        })?;
    let response_body = res
        .json::<Response<T>>()
        .await
        .map_err(|e| IndexerApiError::JsonError {
            source: e,
            url: url.to_string(),
        })?;
    if let Some(errors) = &response_body.errors
        && !errors.is_empty()
    {
        return Err(IndexerApiError::GraphqlError {
            messages: errors.iter().map(|i| i.to_string()).collect(),
            url: url.to_string(),
        });
    }
    Ok(response_body)
}
