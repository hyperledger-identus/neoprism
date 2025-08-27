use graphql_client::{GraphQLQuery, Response};
use identus_apollo::hex::HexStr;
use identus_did_midnight::did::MidnightDid;
use identus_did_midnight::dlt::ContractState;

type HexEncoded = HexStr;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.gql",
    query_path = "graphql/query.gql",
    response_derives = "Debug"
)]
struct ContractStateQuery;

pub async fn get_contract_state(url: &str, did: &MidnightDid) -> Result<ContractState, Box<dyn std::error::Error>> {
    let address_bytes = {
        let mut global_addr = [0u8; 35];
        let network_addr = did.contract_address.as_slice();
        global_addr[2..].copy_from_slice(network_addr);
        global_addr
    };
    let address = HexStr::from(address_bytes).to_string();
    let variables = contract_state_query::Variables { address: Some(address) };
    let request_body = ContractStateQuery::build_query(variables);
    let client = reqwest::Client::new();
    let res = client.post(url).json(&request_body).send().await?;
    let response_body: Response<contract_state_query::ResponseData> = res.json().await?;
    tracing::info!("indexer response: {:#?}", response_body);

    // Error handling and extraction
    if let Some(errors) = response_body.errors {
        if !errors.is_empty() {
            return Err(format!("indexer graphql error: {}", errors[0].message).into());
        }
    }
    let data = response_body
        .data
        .ok_or_else(|| Box::<dyn std::error::Error>::from("indexer response has no data"))?;
    let contract = data
        .contract_action
        .ok_or_else(|| Box::<dyn std::error::Error>::from("contractAction is not found"))?;
    Ok(contract.state.into())
}
