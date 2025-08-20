use std::{
    error::Error,
    fmt::{Debug, Display},
    mem::offset_of,
};

use zerocopy::{ConvertError, FromBytes, IntoBytes, TryFromBytes};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes, TryFromBytes};

use crate::com::MAX_PACKET_SIZE;

const REQUEST_MAGIC: u32 = 0xBC_FEED_AD;

#[derive(TryFromBytes, IntoBytes, Immutable)]
#[repr(u32)]
enum ReqMagicNumInner {
    RequestMagic = REQUEST_MAGIC,
}

#[derive(TryFromBytes, IntoBytes, Immutable)]
#[repr(transparent)]
pub struct RequestMagicNumber(ReqMagicNumInner);
impl RequestMagicNumber {
    pub const fn get() -> Self {
        Self(ReqMagicNumInner::RequestMagic)
    }
}

/// The layout of a Request Header to the WM from a client
#[derive(TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct RequestHeader {
    magic: RequestMagicNumber,
}

/// A Request to ask the WM to Create a new Window
#[derive(FromBytes, IntoBytes, Immutable, Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateWindow {
    flags: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl CreateWindow {
    /// Constructs a new [`CreateWindow`] Request
    pub const fn new(flags: u32, width: u32, height: u32, x: u32, y: u32) -> Self {
        Self {
            flags,
            width,
            height,
            x,
            y,
        }
    }

    pub const fn x(&self) -> u32 {
        self.x
    }

    pub const fn y(&self) -> u32 {
        self.y
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }
}

/// A Request that pings connection with the WM
#[derive(TryFromBytes, IntoBytes, Immutable, Debug, Clone, Copy)]
#[repr(C)]
pub struct Ping;
impl Ping {
    /// Constructs a new [`Self`] Request
    pub const fn new() -> Self {
        Self
    }
}

/// A Request to ask the WM to mark width*height pixels as Damaged (i.e should be updated).
#[derive(FromBytes, IntoBytes, Immutable, Debug, Clone, Copy)]
#[repr(C)]
pub struct DamageWindow {
    /// X Position within the Window
    x: u32,
    /// Y Position within the Window
    y: u32,
    /// Width of the given pixels to draw
    width: u32,
    /// Height of the given pixels to draw
    height: u32,
    /// The ID of the target Window
    win_id: u16,
    __0: u16,
}

impl DamageWindow {
    pub const fn new(win_id: u16, start_x: u32, start_y: u32, width: u32, height: u32) -> Self {
        Self {
            x: start_x,
            y: start_y,
            width,
            height,
            win_id,
            __0: 0,
        }
    }

    pub const fn win_id(&self) -> u16 {
        self.win_id
    }

    pub const fn x(&self) -> u32 {
        self.x
    }

    pub const fn y(&self) -> u32 {
        self.y
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn width(&self) -> u32 {
        self.width
    }
}

#[derive(TryFromBytes, IntoBytes, Immutable, Debug, Clone, Copy)]
#[repr(u16)]
pub enum RequestKind {
    /// See [`CreateWindow`]
    CreateWindow,
    /// See [`DamageWindow`]
    DamageWindow,
    /// See [`Ping`]
    Ping,
}

impl RequestKind {
    /// Returns the size of the internal request data for a given kind
    pub const fn size(self) -> usize {
        match self {
            Self::CreateWindow => size_of::<CreateWindow>(),
            Self::Ping => size_of::<Ping>(),
            Self::DamageWindow => size_of::<DamageWindow>(),
        }
    }
}

pub enum Request {
    CreateWindow(CreateWindow),
    DamageWindow(DamageWindow),
    Ping(Ping),
}

impl Request {
    /// Given a Request convert it to bytes and a kind
    pub fn as_bytes(&self) -> (&[u8], RequestKind) {
        match self {
            Self::CreateWindow(r) => (r.as_bytes(), RequestKind::CreateWindow),
            Self::DamageWindow(d) => (d.as_bytes(), RequestKind::DamageWindow),
            Self::Ping(Ping) => (&[], RequestKind::Ping),
        }
    }
}

type RawRequestData = [u8; MAX_PACKET_SIZE - size_of::<RequestHeader>() - size_of::<RequestKind>()];
/// Describes a Request from the client to the WM
#[derive(TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct RawRequest {
    header: RequestHeader,
    kind: RequestKind,
    data: RawRequestData,
}

#[derive(Debug, Clone, Copy)]
/// An Error while parsing a [`RawRequest`] sent from a client.
pub enum RequestParseErr {
    InvalidMagic,
    InvalidRequestKind,
    InvalidPacketSize,
}

impl Display for RequestParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl Error for RequestParseErr {}

impl RawRequest {
    /// Creates a new request that can be sent to the WM
    pub fn new_valid(req: Request) -> Self {
        let mut data: RawRequestData = unsafe { core::mem::zeroed() };
        let (req_bytes, kind) = req.as_bytes();
        data[..req_bytes.len()].copy_from_slice(req_bytes);

        Self {
            header: RequestHeader {
                magic: RequestMagicNumber::get(),
            },
            kind,
            data,
        }
    }

    /// Converts a reference to self to a reference to bytes (zero-cost conversion)
    pub fn as_bytes(&self) -> &[u8] {
        IntoBytes::as_bytes(self)
    }

    /// Reads and returns the actual Request inside the RawRequest
    pub fn into_request(self) -> Request {
        let bytes = &self.data[..self.kind.size()];

        match self.kind {
            RequestKind::CreateWindow => {
                Request::CreateWindow(match FromBytes::read_from_bytes(&bytes) {
                    Ok(k) => k,
                    Err(_) => unreachable!(),
                })
            }
            RequestKind::DamageWindow => {
                Request::DamageWindow(match FromBytes::read_from_bytes(&bytes) {
                    Ok(k) => k,
                    Err(_) => unreachable!(),
                })
            }
            RequestKind::Ping => Request::Ping(Ping),
        }
    }

    /// Parses and reads given bytes into a RawRequest
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, RequestParseErr> {
        if bytes.len() < size_of::<Self>() {
            return Err(RequestParseErr::InvalidPacketSize);
        }

        let request_bytes = &bytes[..size_of::<Self>()];

        let header_off = offset_of!(Self, header);
        let kind_off = offset_of!(Self, kind);
        let data_off = offset_of!(Self, data);

        let header_bytes = &request_bytes[header_off..header_off + size_of::<RequestHeader>()];
        let kind_bytes = &request_bytes[kind_off..kind_off + size_of::<RequestKind>()];
        let data_bytes = &request_bytes[data_off..data_off + size_of::<RawRequestData>()];

        let header: RequestHeader = match TryFromBytes::try_read_from_bytes(header_bytes) {
            Ok(h) => h,
            Err(ConvertError::Validity(_)) => return Err(RequestParseErr::InvalidMagic),
            Err(ConvertError::Size(_)) => unreachable!(),
        };

        let kind: RequestKind = match TryFromBytes::try_read_from_bytes(kind_bytes) {
            Ok(h) => h,
            Err(ConvertError::Validity(_)) => return Err(RequestParseErr::InvalidRequestKind),
            Err(ConvertError::Size(_)) => unreachable!(),
        };

        Ok(Self {
            header,
            kind,
            data: data_bytes.try_into().unwrap(), /* Size was checked at some point */
        })
    }
}
