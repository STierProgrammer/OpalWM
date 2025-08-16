use std::{
    io::{self, Read, Write},
    sync::{LazyLock, Mutex},
};

use opal_abi::com::{
    RawPacketBytes,
    request::{Ping, RawRequest, Request},
    response::{OkResponse, RawResponse, Response},
};
use safa_api::sockets::UnixSockConnection;

static WM_CONNECTION: LazyLock<Mutex<UnixSockConnection>> = LazyLock::new(|| {
    use safa_api::sockets::{SockKind, UnixSockConnectionBuilder};

    let addr = opal_abi::CONNECT_ABSTRACT_ADDR;
    let mut builder = UnixSockConnectionBuilder::from_abstract_path(addr).unwrap();

    builder.set_type(SockKind::SeqPacket);
    builder
        .connect()
        .map(|k| Mutex::new(k))
        .unwrap_or_else(|_| panic!("Failed to establish connection with the Opal WM at {addr}"))
});

fn send_request(req: Request) -> io::Result<Response> {
    #[repr(align(16))]
    struct Packet(RawPacketBytes);

    let raw = RawRequest::new_valid(req);
    let bytes = raw.as_bytes();
    let mut wm = WM_CONNECTION.lock().expect("Failed to lock WM connection");

    Write::write(&mut *wm, bytes)?;

    let mut packet: Packet = unsafe { core::mem::zeroed() };
    let read = Read::read(&mut *wm, &mut packet.0)?;

    let msg = &packet.0[..read];

    let raw_response = RawResponse::from_bytes(msg);
    let response = raw_response.into_result();
    Ok(response)
}

/// Initializes the client that is going to communicate with the WM
/// Panicks on failure
pub fn init() {
    assert!(send_request(Request::Ping(Ping)).is_ok_and(|o| o == Ok(OkResponse::Success)),)
}
