/// The max size of a packet that can be transferred to and from the WM
pub const MAX_PACKET_SIZE: usize = 256;

/// A Raw unprased packet that is sent to or from the WM
pub type RawPacketBytes = [u8; MAX_PACKET_SIZE];

/// The layout of the requests made to the WM
pub mod request;

/// The layout of the responses the WM can respond with to requests
pub mod response;
