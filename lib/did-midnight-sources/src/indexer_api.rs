use graphql_client::{GraphQLQuery, Response};
use identus_apollo::hex::HexStr;
use identus_did_midnight::did::MidnightDid;

type HexEncoded = HexStr;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.gql",
    query_path = "graphql/query.gql",
    response_derives = "Debug"
)]
struct ContractStateQuery;

pub async fn get_contract_state(url: &str, did: &MidnightDid) -> Result<(), Box<dyn std::error::Error>> {
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

    // TODO: return contract state
    Ok(())
}
