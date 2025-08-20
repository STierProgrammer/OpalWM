use bincode::{Decode, Encode};

use crate::com::packet::{BINCODE_CONFIG, MAX_PACKET_SIZE, PacketParseErr};

/// A Request to ask the WM to Create a new Window
#[derive(Debug, Clone, Copy, Encode, Decode)]
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

/// A Request to ask the WM to mark width*height pixels as Damaged (i.e should be updated).
#[derive(Debug, Clone, Copy, Encode, Decode)]
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

/// The kind of request sent to the WM from a client
#[derive(Debug, Encode, Decode)]
#[repr(u32)]
pub enum RequestKind {
    /// A request to ping the WM (ensures the connection is alive)
    Ping,
    /// See [`CreateWindow`]
    CreateWindow(CreateWindow),
    /// See [`DamageWindow`]
    DamageWindow(DamageWindow),
}

#[derive(Encode, Decode, Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum ReqMagicNumInner {
    RequestMagic = 0xBC_FEED_AD,
}

/// The layout of a Request sent to the WM from a client
#[derive(Debug, Encode, Decode)]
#[repr(C)]
pub struct Request {
    magic: ReqMagicNumInner,
    kind: RequestKind,
}

impl Request {
    /// Constructs a new Request with the given kind.
    pub const fn new(kind: RequestKind) -> Self {
        Self {
            magic: ReqMagicNumInner::RequestMagic,
            kind,
        }
    }

    pub const fn kind(&self) -> &RequestKind {
        &self.kind
    }

    /// Encodes the Request into a byte array and returns the length of the encoded data.
    pub fn encode(self) -> ([u8; MAX_PACKET_SIZE], usize) {
        let mut dst = [0u8; MAX_PACKET_SIZE];
        let len = bincode::encode_into_slice(self, &mut dst, BINCODE_CONFIG)
            .expect("Encoding a Request should never fail");
        (dst, len)
    }

    /// Decodes a byte array into a Request.
    pub fn decode(data: &[u8]) -> Result<Self, PacketParseErr> {
        Ok((bincode::decode_from_slice(data, BINCODE_CONFIG)?).0)
    }
}
