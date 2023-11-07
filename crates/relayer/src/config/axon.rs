use ibc_relayer_types::core::ics24_host::identifier::ChainId;
use serde_derive::{Deserialize, Serialize};
use tendermint_rpc::Url;
use tendermint_rpc::WebSocketClientUrl;

use super::filter::PacketFilter;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AxonChainConfig {
    pub id: ChainId,
    pub websocket_addr: WebSocketClientUrl,
    pub rpc_addr: Url,
    pub contract_address: ethers::types::Address,
    pub transfer_contract_address: ethers::types::Address,
    pub restore_block_count: u64,
    pub key_name: String,
    pub store_prefix: String,
    pub emitter_ckb_url: Url,
    pub emitter_scan_start_block_number: u64,

    #[serde(default)]
    pub packet_filter: PacketFilter,
}
