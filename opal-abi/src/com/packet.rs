use std::{
    error::Error,
    fmt::{Debug, Display},
};

use bincode::{
    config::{Fixint, Limit, LittleEndian},
    error::DecodeError,
};

use crate::com::{request::ReqMagicNumInner, response::Response};

/// The max size of a packet that can be transferred to and from the WM
pub const MAX_PACKET_SIZE: usize = 256;

pub(crate) const BINCODE_CONFIG: bincode::config::Configuration<
    LittleEndian,
    Fixint,
    Limit<MAX_PACKET_SIZE>,
> = bincode::config::standard()
    .with_fixed_int_encoding()
    .with_limit::<MAX_PACKET_SIZE>();

#[derive(Debug, Clone, Copy)]
/// An Error while parsing a packet, either a request or a response.
pub enum PacketParseErr {
    InvalidMagic,
    InvalidPacketKind,
    InvalidPacketSize,
    InvalidPacketData,
}

impl Display for PacketParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl Error for PacketParseErr {}

impl From<bincode::error::DecodeError> for PacketParseErr {
    fn from(value: bincode::error::DecodeError) -> Self {
        match value {
            DecodeError::UnexpectedVariant { type_name, .. }
                if type_name == std::any::type_name::<Response>()
                    || type_name == std::any::type_name::<ReqMagicNumInner>() =>
            {
                PacketParseErr::InvalidMagic
            }
            DecodeError::UnexpectedVariant { .. } => PacketParseErr::InvalidPacketKind,
            DecodeError::ArrayLengthMismatch { .. }
            | DecodeError::LimitExceeded
            | DecodeError::UnexpectedEnd { .. } => PacketParseErr::InvalidPacketSize,
            _ => PacketParseErr::InvalidPacketData,
        }
    }
}
