use std::str::FromStr;
use std::time::Duration;

use crate::error::Error;

use ckb_ics_axon::handler::{IbcChannel as CkbIbcChannel, IbcConnections, IbcPacket};
use ckb_ics_axon::message::{Envelope, MsgType};
use ckb_ics_axon::object::{
    ConnectionEnd as CkbConnectionEnd, Ordering as CkbOrdering, State as CkbState,
};
use ckb_jsonrpc_types::TransactionView;
use ckb_types::packed::WitnessArgs;
use ckb_types::prelude::Entity;
use ibc_relayer_types::core::ics03_connection::connection::{
    ConnectionEnd, IdentifiedConnectionEnd,
};
use ibc_relayer_types::core::ics03_connection::connection::{
    Counterparty as ConnectionCounterparty, State as ConnectionState,
};
use ibc_relayer_types::core::ics04_channel::channel::{
    ChannelEnd, Counterparty as ChannelCounterparty, IdentifiedChannelEnd, Order,
    State as ChannelState,
};
use ibc_relayer_types::core::ics04_channel::version::Version;
use ibc_relayer_types::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};

pub fn extract_channel_end_from_tx(
    tx: TransactionView,
) -> Result<(IdentifiedChannelEnd, CkbIbcChannel), Error> {
    let idx = get_object_idx(&tx, ObjectType::ChannelEnd)?;
    let witness = tx.inner.witnesses.get(idx).unwrap();
    let witness_args = WitnessArgs::from_slice(witness.as_bytes()).unwrap();
    let ckb_channel_end = rlp::decode::<CkbIbcChannel>(witness_args.output_type().as_slice())
        .map_err(|_| Error::extract_chan_tx_error(tx.hash.to_string()))?;

    let channel_end = convert_channel_end(ckb_channel_end.clone())?;

    Ok((channel_end, ckb_channel_end))
}

pub fn extract_ibc_connections_from_tx(tx: TransactionView) -> Result<IbcConnections, Error> {
    let idx = get_object_idx(&tx, ObjectType::IbcConnections)?;
    let witness = tx.inner.witnesses.get(idx).unwrap();
    let witness_args = WitnessArgs::from_slice(witness.as_bytes()).unwrap();
    let ibc_connection_cells = rlp::decode::<IbcConnections>(witness_args.output_type().as_slice())
        .map_err(|_| Error::extract_chan_tx_error(tx.hash.to_string()))?;

    Ok(ibc_connection_cells)
}

pub fn extract_connections_from_tx(
    tx: TransactionView,
) -> Result<(Vec<IdentifiedConnectionEnd>, IbcConnections), Error> {
    let ibc_connection_cell = extract_ibc_connections_from_tx(tx)?;
    let result = ibc_connection_cell
        .connections
        .iter()
        .enumerate()
        .flat_map(|(idx, connection)| convert_connection_end(connection.clone(), idx))
        .collect();
    Ok((result, ibc_connection_cell))
}

pub fn extract_ibc_packet_from_tx(tx: TransactionView) -> Result<IbcPacket, Error> {
    let idx = get_object_idx(&tx, ObjectType::IbcPacket)?;
    let witness = tx.inner.witnesses.get(idx).unwrap();
    let witness_args = WitnessArgs::from_slice(witness.as_bytes()).unwrap();
    let ibc_packet = rlp::decode::<IbcPacket>(witness_args.output_type().as_slice())
        .map_err(|_| Error::extract_chan_tx_error(tx.hash.to_string()))?;
    Ok(ibc_packet)
}

fn navigate(t: MsgType, object_type: ObjectType) -> usize {
    match (t, object_type) {
        (MsgType::MsgChannelOpenInit, ObjectType::IbcConnections) => 0,
        (MsgType::MsgChannelOpenTry, ObjectType::IbcConnections) => 0,
        (MsgType::MsgChannelOpenInit, ObjectType::ChannelEnd) => 1,
        (MsgType::MsgChannelOpenTry, ObjectType::ChannelEnd) => 1,
        (MsgType::MsgChannelOpenAck, ObjectType::ChannelEnd) => 0,
        (MsgType::MsgChannelOpenConfirm, ObjectType::ChannelEnd) => 0,
        (MsgType::MsgChannelCloseInit, ObjectType::ChannelEnd) => 0,
        (MsgType::MsgChannelCloseConfirm, ObjectType::ChannelEnd) => 0,
        (MsgType::MsgSendPacket, ObjectType::ChannelEnd) => 0,
        (MsgType::MsgRecvPacket, ObjectType::ChannelEnd) => 0,
        (MsgType::MsgAckPacket, ObjectType::ChannelEnd) => 0,
        (MsgType::MsgAckOutboxPacket, ObjectType::ChannelEnd) => 0, // only input
        (MsgType::MsgAckInboxPacket, ObjectType::ChannelEnd) => 0,  // only input
        (MsgType::MsgFinishPacket, ObjectType::ChannelEnd) => todo!(),
        (MsgType::MsgTimeoutPacket, ObjectType::ChannelEnd) => todo!(),
        (MsgType::MsgSendPacket, ObjectType::IbcPacket) => 1,
        (MsgType::MsgRecvPacket, ObjectType::IbcPacket) => 1,
        (MsgType::MsgAckPacket, ObjectType::IbcPacket) => 1,
        _ => unreachable!(),
    }
}

fn convert_connection_end(
    connection: CkbConnectionEnd,
    idx: usize,
) -> Result<IdentifiedConnectionEnd, Error> {
    let connection_id = idx.to_string();
    let state = match connection.state {
        CkbState::Unknown => ConnectionState::Uninitialized,
        CkbState::Init => ConnectionState::Init,
        CkbState::OpenTry => ConnectionState::TryOpen,
        CkbState::Open => ConnectionState::Open,
        _ => ConnectionState::Uninitialized,
    };
    let client_id = {
        let s = connection.client_id;
        ClientId::from_str(&s).map_err(|_| Error::ckb_conn_id_invalid(s))
    }?;
    let remote_client_id = {
        let id = connection.counterparty.client_id;
        ClientId::from_str(&id).map_err(|_| Error::ckb_conn_id_invalid(id))
    }?;
    let remote_connection_id = connection
        .counterparty
        .connection_id
        .map(|c| ConnectionId::from_str(&c).unwrap());
    let delay_period = connection.delay_period;
    let result = IdentifiedConnectionEnd {
        connection_id: ConnectionId::from_str(&connection_id).unwrap(),
        connection_end: ConnectionEnd::new(
            state,
            client_id,
            ConnectionCounterparty::new(remote_client_id, remote_connection_id, Default::default()),
            vec![],
            Duration::from_secs(delay_period),
        ),
    };
    Ok(result)
}

fn convert_channel_end(ckb_channel_end: CkbIbcChannel) -> Result<IdentifiedChannelEnd, Error> {
    let state = match ckb_channel_end.state {
        CkbState::Unknown => ChannelState::Uninitialized,
        CkbState::Init => ChannelState::Init,
        CkbState::OpenTry => ChannelState::TryOpen,
        CkbState::Open => ChannelState::Open,
        CkbState::Closed => ChannelState::Closed,
        CkbState::Frozen => panic!(),
    };
    let ordering = match ckb_channel_end.order {
        CkbOrdering::Unknown => Order::None,
        CkbOrdering::Unordered => Order::Unordered,
        CkbOrdering::Ordered => Order::Ordered,
    };
    let remote_port_id = PortId::from_str(&ckb_channel_end.counterparty.port_id)
        .map_err(|_| Error::convert_channel_end())?;
    let remote_channel_id = ChannelId::from_str(&ckb_channel_end.counterparty.channel_id)
        .map_err(|_| Error::convert_channel_end())?;
    let remote = ChannelCounterparty {
        port_id: remote_port_id,
        channel_id: Some(remote_channel_id),
    };
    let connection_hops = {
        ckb_channel_end
            .connection_hops
            .into_iter()
            .flat_map(|c| {
                let conn_id_str = c.to_string();
                ConnectionId::from_str(&conn_id_str).map_err(|_| Error::convert_channel_end())
            })
            .collect::<Vec<_>>()
    };
    let channel_end = ChannelEnd {
        state,
        ordering,
        remote,
        connection_hops,
        version: Version::empty(),
    };

    let port_id =
        PortId::from_str(&ckb_channel_end.port_id).map_err(|_| Error::convert_channel_end())?;

    let channel_id = ChannelId::from_str(&ckb_channel_end.num.to_string())
        .map_err(|_| Error::convert_channel_end())?;

    let result = IdentifiedChannelEnd {
        port_id,
        channel_id,
        channel_end,
    };
    Ok(result)
}

enum ObjectType {
    ChannelEnd,
    IbcConnections,
    IbcPacket,
}

fn get_object_idx(tx: &TransactionView, object_type: ObjectType) -> Result<usize, Error> {
    let msg = tx
        .inner
        .witnesses
        .last()
        .ok_or(Error::extract_chan_tx_error(tx.hash.to_string()))?;

    let bytes = msg.as_bytes().to_vec();

    let envelope = rlp::decode::<Envelope>(&bytes)
        .map_err(|_| Error::extract_chan_tx_error(tx.hash.to_string()))?;

    Ok(navigate(envelope.msg_type, object_type))
}
