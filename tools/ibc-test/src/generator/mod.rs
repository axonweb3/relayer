use ckb_types::{h256, H256};
use create_connection::generate_create_connection;
use deploy_conn_chan::generate_deploy_conn_chan;
use deploy_packet_metadata::generate_deploy_packet_metadata;

use self::test_config::{generate_consts_file, generate_test_config};

mod create_connection;
mod deploy_conn_chan;
mod deploy_packet_metadata;
mod test_config;
mod utils;

pub const PRIVKEY: &str = "63d86723e08f0f813a36ce6aa123bb2289d90680ae1e99d4de8cdb334553f24d";
pub const GENESIS_TXHASH: H256 =
    h256!("0x227de871ce6ab120a67960f831b04148bf79b4e56349dde7a8001f93191736ed");

#[ignore]
#[test]
fn generate() {
    let conn_chan_attr = generate_deploy_conn_chan();
    let packet_metadata_attr = generate_deploy_packet_metadata(&conn_chan_attr);
    let (_, _) = generate_create_connection(&conn_chan_attr, &packet_metadata_attr);
    generate_test_config(&conn_chan_attr, &packet_metadata_attr);
    generate_consts_file(&conn_chan_attr, &packet_metadata_attr);
}
