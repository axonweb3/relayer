mod chan;
mod conn;

use std::{borrow::Borrow, cell::Ref, collections::HashMap};

use chan::*;
use conn::*;

use crate::{config::ckb4ibc::ChainConfig, error::Error, keyring::Secp256k1KeyPair};
use ckb_ics_axon::{
    handler::{IbcChannel, IbcConnections},
    message::Envelope,
};
use ckb_types::core::TransactionView;
use ckb_types::packed::{Byte32, CellInput, OutPoint};
use ibc_proto::google::protobuf::Any;
use ibc_relayer_types::{
    core::ics03_connection::msgs::{
        conn_open_ack::MsgConnectionOpenAck, conn_open_ack::TYPE_URL as CONN_OPEN_ACK_TYPE_URL,
        conn_open_confirm::MsgConnectionOpenConfirm,
        conn_open_confirm::TYPE_URL as CONN_OPEN_CONFIRM_TYPE_URL,
        conn_open_init::MsgConnectionOpenInit, conn_open_init::TYPE_URL as CONN_OPEN_INIT_TYPE_URL,
        conn_open_try::MsgConnectionOpenTry, conn_open_try::TYPE_URL as CONN_OPEN_TRY_TYPE_URL,
    },
    core::{
        ics04_channel::{
            msgs::{
                acknowledgement::MsgAcknowledgement,
                acknowledgement::TYPE_URL as ACK_TYPE_URL,
                chan_close_init::MsgChannelCloseInit,
                chan_close_init::TYPE_URL as CHAN_CLOSE_INIT_TYPE_URL,
                chan_open_ack::MsgChannelOpenAck,
                chan_open_ack::TYPE_URL as CHAN_OPEN_ACK_TYPE_URL,
                chan_open_confirm::MsgChannelOpenConfirm,
                chan_open_confirm::TYPE_URL as CHAN_OPEN_CONFIRM_TYPE_URL,
                chan_open_init::MsgChannelOpenInit,
                chan_open_init::TYPE_URL as CHAN_OPEN_INIT_TYPE_URL,
                chan_open_try::MsgChannelOpenTry,
                chan_open_try::TYPE_URL as CHAN_OPEN_TRY_TYPE_URL,
                recv_packet::{MsgRecvPacket, TYPE_URL as RECV_PACKET_TYPE_URL},
            },
            packet::Sequence,
        },
        ics24_host::identifier::{ChannelId, PortId},
    },
    tx_msg::Msg,
};

use super::utils::get_script_hash;

pub trait MsgToTxConverter {
    fn get_key(&self) -> &Secp256k1KeyPair;

    fn get_ibc_connections(&self) -> IbcConnections;

    fn get_ibc_connections_input(&self) -> CellInput;

    fn get_ibc_channel(&self, id: &ChannelId) -> IbcChannel;

    fn get_ibc_channel_input(&self, channel_id: &ChannelId, port_id: &PortId) -> CellInput;

    fn get_client_outpoint(&self) -> OutPoint;

    fn get_channel_code_hash(&self) -> Byte32;

    fn get_packet_code_hash(&self) -> Byte32;

    fn get_connection_code_hash(&self) -> Byte32;

    fn get_client_id(&self) -> [u8; 32];

    fn get_packet_cell_input(&self, chan: ChannelId, port: PortId, seq: Sequence) -> CellInput;

    fn get_packet_owner(&self) -> [u8; 32];
}

pub struct Converter<'a> {
    pub channel_input_data: Ref<'a, HashMap<(ChannelId, PortId), CellInput>>,
    pub channel_cache: Ref<'a, HashMap<ChannelId, IbcChannel>>,
    pub connection_cache: Ref<'a, Option<(IbcConnections, CellInput)>>,
    pub packet_input_data: Ref<'a, HashMap<(ChannelId, PortId, Sequence), CellInput>>,
    pub config: &'a ChainConfig,
    pub client_outpoint: &'a OutPoint,
    pub packet_owner: [u8; 32],
}

impl<'a> MsgToTxConverter for Converter<'a> {
    fn get_key(&self) -> &Secp256k1KeyPair {
        todo!()
    }

    fn get_ibc_connections(&self) -> IbcConnections {
        self.connection_cache.borrow().as_ref().unwrap().0.clone()
    }

    fn get_ibc_connections_input(&self) -> CellInput {
        self.connection_cache.borrow().as_ref().unwrap().1.clone()
    }

    fn get_ibc_channel(&self, channel_id: &ChannelId) -> IbcChannel {
        self.channel_cache.get(channel_id).unwrap().clone()
    }

    fn get_ibc_channel_input(&self, channel_id: &ChannelId, port_id: &PortId) -> CellInput {
        self.channel_input_data
            .get(&(channel_id.clone(), port_id.clone()))
            .unwrap()
            .clone()
    }

    fn get_client_outpoint(&self) -> OutPoint {
        self.client_outpoint.clone()
    }

    fn get_channel_code_hash(&self) -> Byte32 {
        get_script_hash(self.config.channel_type_args.clone())
    }

    fn get_packet_code_hash(&self) -> Byte32 {
        get_script_hash(self.config.packet_type_args.clone())
    }

    fn get_connection_code_hash(&self) -> Byte32 {
        get_script_hash(self.config.connection_type_args.clone())
    }

    fn get_client_id(&self) -> [u8; 32] {
        self.config.client_id
    }

    fn get_packet_cell_input(
        &self,
        channel_id: ChannelId,
        port_id: PortId,
        sequence: Sequence,
    ) -> CellInput {
        self.packet_input_data
            .get(&(channel_id, port_id, sequence))
            .unwrap()
            .clone()
    }

    fn get_packet_owner(&self) -> [u8; 32] {
        self.packet_owner
    }
}

// Return a transaction which needs to be added relayer's input in it and to be signed.
pub fn convert_msg_to_ckb_tx<C: MsgToTxConverter>(
    msg: Any,
    converter: &C,
) -> Result<(TransactionView, Envelope, u64), Error> {
    match msg.type_url.as_str() {
        // connection
        CONN_OPEN_INIT_TYPE_URL => {
            let msg = MsgConnectionOpenInit::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CONN_OPEN_INIT_TYPE_URL.to_string(), e))?;
            convert_conn_open_init_to_tx(msg, converter)
        }
        CONN_OPEN_TRY_TYPE_URL => {
            let msg = MsgConnectionOpenTry::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CONN_OPEN_TRY_TYPE_URL.to_string(), e))?;
            convert_conn_open_try_to_tx(msg, converter)
        }
        CONN_OPEN_ACK_TYPE_URL => {
            let msg = MsgConnectionOpenAck::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CONN_OPEN_ACK_TYPE_URL.to_string(), e))?;
            convert_conn_open_ack_to_tx(msg, converter)
        }
        CONN_OPEN_CONFIRM_TYPE_URL => {
            let msg = MsgConnectionOpenConfirm::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CONN_OPEN_CONFIRM_TYPE_URL.to_string(), e))?;
            convert_conn_open_confirm_to_tx(msg, converter)
        }
        // chanel
        CHAN_OPEN_INIT_TYPE_URL => {
            let msg = MsgChannelOpenInit::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CHAN_OPEN_INIT_TYPE_URL.to_string(), e))?;
            convert_chan_open_init_to_tx(msg, converter)
        }
        CHAN_OPEN_TRY_TYPE_URL => {
            let msg = MsgChannelOpenTry::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CHAN_OPEN_TRY_TYPE_URL.to_string(), e))?;
            convert_chan_open_try_to_tx(msg, converter)
        }
        CHAN_OPEN_ACK_TYPE_URL => {
            let msg = MsgChannelOpenAck::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CHAN_OPEN_ACK_TYPE_URL.to_string(), e))?;
            convert_chan_open_ack_to_tx(msg, converter)
        }
        CHAN_OPEN_CONFIRM_TYPE_URL => {
            let msg = MsgChannelOpenConfirm::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CHAN_OPEN_CONFIRM_TYPE_URL.to_string(), e))?;
            convert_chan_open_confirm_to_tx(msg, converter)
        }
        CHAN_CLOSE_INIT_TYPE_URL => {
            let msg = MsgChannelCloseInit::from_any(msg)
                .map_err(|e| Error::protobuf_decode(CHAN_CLOSE_INIT_TYPE_URL.to_string(), e))?;
            convert_chan_close_init_to_tx(msg, converter)
        }
        // packet
        RECV_PACKET_TYPE_URL => {
            let msg = MsgRecvPacket::from_any(msg)
                .map_err(|e| Error::protobuf_decode(RECV_PACKET_TYPE_URL.to_string(), e))?;
            convert_recv_packet_to_tx(msg, converter)
        }
        ACK_TYPE_URL => {
            let msg = MsgAcknowledgement::from_any(msg)
                .map_err(|e| Error::protobuf_decode(ACK_TYPE_URL.to_string(), e))?;
            convert_ack_packet_to_tx(msg, converter)
        }
        _ => todo!(),
    }
}
