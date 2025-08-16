use zerocopy::{IntoBytes, TryFromBytes};
use zerocopy_derive::{Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::com::{MAX_PACKET_SIZE, request::RequestParseErr};

const RESPONSE_OK_MAGIC: u32 = 0xBC_F00D_AD;
const RESPONSE_ERR_MAGIC: u32 = 0xBC_DEAD_AD;

type RawResponseData = [u8; MAX_PACKET_SIZE - size_of::<ResponseError>() - 4];

/// A Response Error
#[derive(Debug, Clone, Copy, IntoBytes, TryFromBytes, Immutable, PartialEq, Eq)]
#[repr(u32)]
pub enum ResponseError {
    InvalidMagic,
    InvalidRequestKind,
    PacketTooShort,
    InvalidData,
}

impl From<RequestParseErr> for ResponseError {
    fn from(value: RequestParseErr) -> Self {
        match value {
            RequestParseErr::InvalidMagic => ResponseError::InvalidMagic,
            RequestParseErr::InvalidPacketSize => ResponseError::PacketTooShort,
            RequestParseErr::InvalidRequestKind => ResponseError::InvalidRequestKind,
        }
    }
}

/// Represents a Raw Ok Response kind, should be converted to an [`OkResponse`]
#[derive(TryFromBytes, IntoBytes, Immutable, Clone, Copy)]
#[repr(u32)]
pub enum OkResponseKind {
    Success,
}

impl OkResponseKind {
    pub const fn size(&self) -> usize {
        match self {
            Self::Success => 0,
        }
    }
}

const _: () = assert!(size_of::<OkResponseKind>() == size_of::<ResponseError>());

#[derive(Debug, PartialEq, Eq)]
/// Represents an Ok response sent by the WM as a reply to a Request
pub enum OkResponse {
    Success,
}

impl OkResponse {
    const fn as_bytes(&self) -> (&[u8], OkResponseKind) {
        match self {
            Self::Success => (&[], OkResponseKind::Success),
        }
    }
}

#[derive(TryFromBytes, IntoBytes, Immutable, Clone, Copy)]
#[repr(C, align(4))]
pub struct RawOkResponse {
    kind: OkResponseKind,
    data: RawResponseData,
}

#[derive(TryFromBytes, IntoBytes, Immutable, Clone, Copy)]
#[repr(C)]
pub struct RawErrResponse {
    kind: ResponseError,
    __: RawResponseData,
}

const _: () = assert!(size_of::<RawErrResponse>() == size_of::<RawOkResponse>());

#[derive(TryFromBytes, IntoBytes, Immutable, KnownLayout, Clone, Copy)]
#[repr(u32)]
pub enum RawResponse {
    Ok(RawOkResponse) = RESPONSE_OK_MAGIC,
    Err(RawErrResponse) = RESPONSE_ERR_MAGIC,
}

impl RawErrResponse {
    /// Consumes Raw Self into an non raw Error
    pub const fn into_error(self) -> ResponseError {
        self.kind
    }
}

impl RawOkResponse {
    /// Converts Raw Self to a non raw Response
    pub fn consume(self) -> OkResponse {
        let size = self.kind.size();
        let bytes = &self.data[..size];
        _ = bytes;

        match self.kind {
            OkResponseKind::Success => OkResponse::Success,
        }
    }
}

/// The non-Raw version of [`RawResponse`]
pub type Response = Result<OkResponse, ResponseError>;

impl RawResponse {
    /// Converts a RawResponse into a Rust Result
    pub fn into_result(self) -> Response {
        match self {
            RawResponse::Err(e) => Err(e.into_error()),
            RawResponse::Ok(k) => Ok(k.consume()),
        }
    }

    /// Convert a given a result into Self
    pub fn from_result(result: Response) -> Self {
        match result {
            Ok(k) => {
                let mut data: RawResponseData = unsafe { core::mem::zeroed() };

                let (bytes, kind) = k.as_bytes();
                data[..bytes.len()].copy_from_slice(bytes);

                Self::Ok(RawOkResponse { kind, data })
            }
            Err(e) => Self::Err(RawErrResponse {
                kind: e,
                __: unsafe { core::mem::zeroed() },
            }),
        }
    }

    /// Converts a reference to self into a reference to bytes of self
    pub fn into_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
    /// Converts a reference to bytes into a reference to self
    ///
    /// Panicks on failure, and that will happen
    /// if the reference to bytes was unaligned
    /// or if the bytes point to invalid data,
    /// or if bytes.len() is smaller than size_of::<Self>()
    pub fn from_bytes(bytes: &[u8]) -> &Self {
        TryFromBytes::try_ref_from_bytes(&bytes[..size_of::<Self>()])
            .expect("Failed to parse response")
    }
}

impl From<Response> for RawResponse {
    fn from(value: Response) -> Self {
        RawResponse::from_result(value)
    }
}
