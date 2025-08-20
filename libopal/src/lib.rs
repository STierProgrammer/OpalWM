use std::{
    io::{self, Read, Write},
    sync::{LazyLock, Mutex},
};

use opal_abi::com::{
    packet::MAX_PACKET_SIZE,
    request::{Request, RequestKind},
    response::{OkResponse, Response},
};
use safa_api::sockets::UnixSockConnection;

pub mod window;

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

pub(crate) fn send_request(req: RequestKind) -> io::Result<Response> {
    let request = Request::new(req);
    let (bytes, len) = request.encode();
    let mut wm = WM_CONNECTION.lock().expect("Failed to lock WM connection");

    Write::write(&mut *wm, &bytes[..len])?;

    let mut packet: [u8; MAX_PACKET_SIZE] = [0u8; MAX_PACKET_SIZE];
    let read = Read::read(&mut *wm, &mut packet)?;

    let msg = &packet[..read];

    let response = Response::decode(msg).expect("Couldn't Parse WM's response");
    Ok(response)
}

/// Initializes the client that is going to communicate with the WM
/// Panicks on failure
pub fn init() {
    assert!(
        send_request(RequestKind::Ping).is_ok_and(|o| o == Response::Ok(OkResponse::Success)),
        "Ping request, responded with an error"
    )
}
