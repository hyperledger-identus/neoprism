use std::str::FromStr;

use identus_apollo::hex::HexStr;
use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::proto::MessageExt;
use identus_did_prism::proto::prism::PrismObject;

#[test]
fn decode_prism_object() {
    let hex_chunk = [
        "2296011293010a086d61737465722d30124630440220338f7677eac8cd5c462e6b6c33583e805e16e7d16693c4ccaafc676fa622f3eb02207bfa6130fc7c84c9",
        "c096f8991cb9ea0ef6b0f1bb5ba4a3d8683677c8531304d81a3f0a3d0a3b12390a086d61737465722d3010014a2b0a074564323535313912208e15514e5dc189",
        "0d63a1c69a9db5638709556a1f432d2f03501740ba7fe9ec2d",
    ];
    let hex_str = hex_chunk.join("");
    let bytes = HexStr::from_str(&hex_str).expect("invalid hex string").to_bytes();
    let prism_object = PrismObject::decode(&bytes).expect("invalid protobuf message bytes");

    let dids = prism_object
        .block_content
        .operations
        .iter()
        .flat_map(|i| {
            i.operation
                .as_ref()
                .and_then(|o| CanonicalPrismDid::from_operation(o).ok())
        })
        .collect::<Vec<CanonicalPrismDid>>()
        .into_iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();

    assert_eq!(
        dids,
        ["did:prism:13ae40733718a27b7ae0e99cf686ba62b9b8ca1203848cbf3720381c4e6081d9"]
    );
}
