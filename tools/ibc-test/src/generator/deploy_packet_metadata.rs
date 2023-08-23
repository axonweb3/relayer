use axon_types::metadata::{Metadata, ValidatorList};
use ckb_hash::new_blake2b;
use ckb_sdk::{
    traits::SecpCkbRawKeySigner,
    unlock::{ScriptSigner, SecpSighashScriptSigner},
    ScriptGroup, ScriptGroupType,
};
use ckb_types::{
    core::{ScriptHashType, TransactionView},
    h256,
    packed::{CellInput, CellOutput, OutPoint, Script, ScriptOpt},
    prelude::*,
    H256,
};

use crate::generator::{
    utils::{get_lock_script, get_secp256k1_cell_dep, wrap_rpc_request_and_save},
    PRIVKEY,
};

use super::deploy_conn_chan::ConnChanAttribute;

pub struct PacketMetataAttribute {
    pub tx_hash: H256,
    pub packet_type_args: H256,
    pub packet_code_hash: H256,
    pub packet_index: usize,
    pub metadata_index: usize,
    pub metadata_type_args: H256,
    pub balance_index: usize,
}

pub fn generate_deploy_packet_metadata(attribute: &ConnChanAttribute) -> PacketMetataAttribute {
    let input = CellInput::new_builder()
        .previous_output(
            OutPoint::new_builder()
                .tx_hash(attribute.tx_hash.pack())
                .index(attribute.balance_index.pack())
                .build(),
        )
        .build();

    let (lock_script, secret_key) = get_lock_script(PRIVKEY);

    let mut blake_2b = new_blake2b();
    blake_2b.update(input.as_slice());
    blake_2b.update(0u64.to_le_bytes().as_slice());
    let mut type_args = [0; 32];
    blake_2b.finalize(&mut type_args);
    println!("packet type args: {:?}", hex::encode(type_args));
    let packet_type_args: H256 = type_args.into();

    let mut blake_2b = new_blake2b();
    blake_2b.update(input.as_slice());
    blake_2b.update(1u64.to_le_bytes().as_slice());
    let mut type_2_args = [0; 32];
    blake_2b.finalize(&mut type_2_args);
    println!("client type args: {:?}", hex::encode(type_2_args));
    let metadata_type_args: H256 = type_2_args.into();
    // let metadata_type_args: H256 = type_2_args.into();

    let packet_type_script = Script::new_builder()
        .code_hash(
            h256!("0x00000000000000000000000000000000000000000000000000545950455f4944").pack(),
        )
        .hash_type(ScriptHashType::Type.into())
        .args(type_args.as_slice().pack())
        .build();

    println!(
        "packet code hash: {}",
        packet_type_script.calc_script_hash()
    );
    let packet_code_hash: H256 = packet_type_script.calc_script_hash().unpack();

    let packet_output = CellOutput::new_builder()
        .type_(
            ScriptOpt::new_builder()
                .set(Some(packet_type_script))
                .build(),
        )
        .lock(lock_script.clone())
        .capacity(200_000_000_000_000u64.pack())
        .build();

    let mock_metadata_script = Script::new_builder()
        .code_hash(
            h256!("0x00000000000000000000000000000000000000000000000000545950455f4944").pack(),
        )
        .hash_type(ScriptHashType::Type.into())
        .args(type_2_args.as_slice().pack())
        .build();

    let metadata = Metadata::new_builder()
        .validators(ValidatorList::new_builder().build())
        .build();

    let metadata_output = CellOutput::new_builder()
        .lock(lock_script.clone())
        .type_(
            ScriptOpt::new_builder()
                .set(Some(mock_metadata_script))
                .build(),
        )
        .capacity(100_000_000_000u64.pack())
        .build();

    let change_output = CellOutput::new_builder()
        .lock(lock_script.clone())
        .capacity(700_000_000_000_000u64.pack())
        .build();

    let signer =
        SecpSighashScriptSigner::new(Box::new(SecpCkbRawKeySigner::new_with_secret_keys(vec![
            secret_key,
        ])));
    let empty_data = "0x".as_bytes().to_vec().pack();
    let tx = TransactionView::new_advanced_builder()
        .cell_dep(get_secp256k1_cell_dep())
        .input(input)
        .output(packet_output)
        .output(metadata_output)
        .output(change_output)
        .output_data(std::fs::read("./contracts/ics-packet").unwrap().pack())
        .output_data(metadata.as_slice().pack())
        .output_data(empty_data)
        .build();

    let tx = signer
        .sign_tx(
            &tx,
            &ScriptGroup {
                script: lock_script,
                group_type: ScriptGroupType::Lock,
                input_indices: vec![0],
                output_indices: vec![2],
            },
        )
        .unwrap();

    let tx_hash = wrap_rpc_request_and_save(tx, "./txs/deploy_packet_metadata.json");

    PacketMetataAttribute {
        tx_hash,
        packet_type_args,
        packet_code_hash,
        metadata_type_args,
        packet_index: 0,
        metadata_index: 1,
        balance_index: 2,
    }
}
