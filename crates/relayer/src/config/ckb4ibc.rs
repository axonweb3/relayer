use ckb_types::H256;
use ibc_relayer_types::core::{
    ics02_client::client_type::ClientType,
    ics24_host::identifier::{ChainId, ClientId},
};
use serde::ser::SerializeMap;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use tendermint_rpc::Url;

use crate::error::Error;

use super::filter::PacketFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightClientItem {
    pub chain_id: ChainId,
    pub client_cell_type_args: H256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub id: ChainId,
    pub ckb_rpc: Url,
    pub ckb_indexer_rpc: Url,
    pub key_name: String,
    pub store_prefix: String,

    pub connection_type_args: H256,
    pub channel_type_args: H256,
    pub packet_type_args: H256,

    #[serde(default)]
    pub packet_filter: PacketFilter,

    #[serde(serialize_with = "light_client_serialize")]
    pub onchain_light_clients: HashMap<ClientType, LightClientItem>,
}

impl ChainConfig {
    pub fn lc_chain_id(&self, client_id: &String) -> Result<ChainId, Error> {
        let chain_id = self
            .onchain_light_clients
            .iter()
            .find_map(|(_, v)| {
                if hex::encode(&v.client_cell_type_args) == client_id.as_str() {
                    Some(v.chain_id.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::other_error(format!(
                "config.toml missing {client_id}"
            )))?;
        Ok(chain_id)
    }

    pub fn lc_client_type(&self, client_id: &str) -> Result<ClientType, Error> {
        let client_type = self
            .onchain_light_clients
            .iter()
            .find_map(|(k, v)| {
                if hex::encode(&v.client_cell_type_args) == client_id {
                    Some(*k)
                } else {
                    None
                }
            })
            .ok_or(Error::other_error(format!(
                "config.toml missing client_id {client_id}"
            )))?;
        Ok(client_type)
    }

    pub fn lc_client_id(&self, client_type: ClientType) -> Result<ClientId, Error> {
        let client_type_args = self.lc_client_type_args(client_type)?;
        let client_id = hex::encode(client_type_args).parse().unwrap();
        Ok(client_id)
    }

    pub fn lc_client_type_args(&self, client_type: ClientType) -> Result<[u8; 32], Error> {
        let (_, item) = self
            .onchain_light_clients
            .iter()
            .find(|(v, _)| **v == client_type)
            .ok_or(Error::other_error(format!(
                "config.toml missing client_type {client_type}"
            )))?;
        Ok(item.client_cell_type_args.clone().into())
    }
}

// it's only workable for serializing `onchain_light_clients` filed into JSON string,
// especially for passing config test cases
fn light_client_serialize<S: serde::Serializer>(
    item: &HashMap<ClientType, LightClientItem>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut map = s.serialize_map(Some(item.len()))?;
    item.iter()
        .try_for_each(|(k, v)| map.serialize_entry(k.as_str(), v))?;
    map.end()
}
