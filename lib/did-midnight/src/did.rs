use std::str::FromStr;

use identus_apollo::hex::HexStr;
use identus_did_core::Did;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::{Display, EnumString};

use crate::error::Error;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display, derive_more::Debug)]
pub enum MidnightNetwork {
    #[strum(serialize = "undeployed")]
    Undeployed,
    #[strum(serialize = "devnet")]
    Devnet,
    #[strum(serialize = "testnet")]
    Testnet,
    #[strum(serialize = "mainnet")]
    Mainnet,
}

impl MidnightNetwork {
    pub fn as_u8_repr(&self) -> u8 {
        match self {
            MidnightNetwork::Undeployed => 0,
            MidnightNetwork::Devnet => 1,
            MidnightNetwork::Testnet => 2,
            MidnightNetwork::Mainnet => 3,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, derive_more::Debug, derive_more::Display)]
#[debug("{}", identus_apollo::hex::HexStr::from(_0))]
#[display("{}", identus_apollo::hex::HexStr::from(_0))]
pub struct MidnightContractAddress([u8; 34]);

impl MidnightContractAddress {
    pub fn as_slice(&self) -> &[u8; 34] {
        &self.0
    }
}

impl FromStr for MidnightContractAddress {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        if s.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(Error::InvalidAddressCase);
        }
        let bytes = HexStr::from_str(s)?.to_bytes();
        let bytes: [u8; 34] = bytes.as_slice().try_into().map_err(|_| Error::InvalidAddressLength {
            found: bytes.len(),
            expected: 34,
        })?;
        Ok(MidnightContractAddress(bytes))
    }
}

impl Serialize for MidnightContractAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MidnightContractAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        MidnightContractAddress::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, derive_more::Debug)]
#[display("did:midnight:{network}:{contract_address}")]
#[debug("did:midnight:{network}:{contract_address}")]
pub struct MidnightDid {
    network: MidnightNetwork,
    contract_address: MidnightContractAddress,
}

impl MidnightDid {
    pub fn method(&self) -> &'static str {
        "midnight"
    }

    pub fn network(&self) -> MidnightNetwork {
        self.network
    }

    pub fn contract_address(&self) -> &MidnightContractAddress {
        &self.contract_address
    }

    pub fn to_did(&self) -> Did {
        let s = self.to_string();
        Did::from_str(&s).expect("MidnightDid does not construct a valid DID syntax")
    }

    pub fn global_contract_address(&self) -> [u8; 35] {
        let network_addr = self.contract_address.as_slice();
        let network_byte: u8 = self.network().as_u8_repr();
        let mut global_addr = [0u8; 35];
        global_addr[0] = network_byte;
        global_addr[1..].copy_from_slice(network_addr);
        global_addr
    }
}

impl FromStr for MidnightDid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 4 {
            return Err(Error::InvalidSegmentCount { found: parts.len() });
        }
        if parts[0] != "did" {
            return Err(Error::InvalidDidSyntax { input: s.to_string() });
        }
        if parts[1] != "midnight" {
            return Err(Error::InvalidMethod {
                method: parts[1].to_string(),
            });
        }
        let network = MidnightNetwork::from_str(parts[2])?;
        let contract_address = MidnightContractAddress::from_str(parts[3])?;
        Ok(MidnightDid {
            network,
            contract_address,
        })
    }
}
