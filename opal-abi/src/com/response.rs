use zerocopy::{FromBytes, IntoBytes, TryFromBytes};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::com::{MAX_PACKET_SIZE, request::RequestParseErr};

const RESPONSE_OK_MAGIC: u32 = 0xA1E_F00D_D;
const RESPONSE_EVENT_MAGIC: u32 = 0xAD_FEED_BC;
const RESPONSE_ERR_MAGIC: u32 = 0xBAD_F00D_C;

type RawResponseData = [u8; MAX_PACKET_SIZE - size_of::<ResponseError>() - 4];

/// A Response Error
#[derive(Debug, Clone, Copy, IntoBytes, TryFromBytes, Immutable, PartialEq, Eq)]
#[repr(u32)]
pub enum ResponseError {
    InvalidMagic,
    InvalidRequestKind,
    PacketTooShort,
    InvalidData,
    UnknownFatalError,
    UnknownWindow,
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

#[derive(Debug, FromBytes, IntoBytes, Immutable, Clone, Copy, PartialEq, Eq)]
/// Response of [`super::request::CreateWindow`]
pub struct CreateWindowResp {
    /// The created window's shared memory key, it can be used to write to the window's pixels.
    shm_key: usize,
    win_id: u16,
    __0: u16,
    __1: u32,
}

impl CreateWindowResp {
    /// The created window's ID
    pub const fn window_id(&self) -> u16 {
        self.win_id
    }

    pub const fn shm_key(&self) -> usize {
        self.shm_key
    }

    pub const fn new(win_id: u16, shm_key: usize) -> Self {
        Self {
            win_id,
            shm_key,
            __0: 0,
            __1: 0,
        }
    }
}

/// Represents a Raw Ok Response kind, should be converted to an [`OkResponse`]
#[derive(TryFromBytes, IntoBytes, Immutable, Clone, Copy)]
#[repr(u32)]
pub enum OkResponseKind {
    Success,
    // See [`CreateWindowResp`]
    WindowCreated,
}

impl OkResponseKind {
    pub const fn size(&self) -> usize {
        match self {
            Self::Success => 0,
            Self::WindowCreated => size_of::<CreateWindowResp>(),
        }
    }
}

const _: () = assert!(size_of::<OkResponseKind>() == size_of::<ResponseError>());

#[derive(Debug, PartialEq, Eq)]
/// Represents an Ok response sent by the WM as a reply to a Request
pub enum OkResponse {
    Success,
    WindowCreated(CreateWindowResp),
}

impl OkResponse {
    fn as_bytes(&self) -> (&[u8], OkResponseKind) {
        match self {
            Self::Success => (&[], OkResponseKind::Success),
            Self::WindowCreated(win) => (win.as_bytes(), OkResponseKind::WindowCreated),
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

        match self.kind {
            OkResponseKind::Success => OkResponse::Success,
            OkResponseKind::WindowCreated => {
                OkResponse::WindowCreated(CreateWindowResp::read_from_bytes(bytes).unwrap())
            }
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
