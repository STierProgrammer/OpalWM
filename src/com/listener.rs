use std::{
    io::ErrorKind,
    process::{Command, Stdio},
};

use opal_abi::com::{
    request::RequestKind,
    response::{CreateWindowResp, OkResponse, Response, error::ResponseError},
};
use safa_api::sockets::{SockKind, UnixListenerBuilder, UnixSockConnection};

use crate::{
    com::{ClientComPipe, ReadError},
    dlog, elog,
    framebuffer::Pixel,
    log, logging,
    window::{self, Window, WindowKind},
};

fn spawn_hello() {
    Command::new("sys:/bin/hello_world")
        .stdout(Stdio::from(logging::console_clone()))
        .stderr(Stdio::from(logging::console_clone()))
        .stdin(Stdio::from(logging::console_clone()))
        .spawn()
        .expect("Failed to spawn test Process");
}

fn handle_connect(connection: UnixSockConnection) {
    dlog!("Handling a new connection");
    let mut pipe = ClientComPipe::new(connection);

    loop {
        dlog!("Waiting for a Request");

        let response = match pipe.read_request() {
            Ok(req) => match req.kind() {
                RequestKind::CreateWindow(request) => {
                    let height = request.height() as usize;
                    let width = request.width() as usize;
                    let pos_x = request.x() as usize;
                    let pos_y = request.y() as usize;

                    let window = Window::new_filled_with(
                        pos_x,
                        pos_y,
                        width,
                        height,
                        Pixel::from_rgba(0, 0, 0, 0xFF),
                    );

                    let shm_key = *window.shm_key();
                    window::add_window(window, WindowKind::Normal)
                        .map(|id| {
                            dlog!("Added Window {id}, with the SHM Key {shm_key} for a client");
                            CreateWindowResp::new(id, shm_key)
                        })
                        .map(OkResponse::WindowCreated)
                        .ok_or(ResponseError::UnknownFatalError)
                }
                RequestKind::DamageWindow(damage) => window::damage_window(
                    damage.win_id(),
                    damage.x() as usize,
                    damage.y() as usize,
                    damage.width() as usize,
                    damage.height() as usize,
                )
                .map(|()| OkResponse::Success)
                .map_err(|()| ResponseError::UnknownWindow),
                RequestKind::Ping => Ok(OkResponse::Success),
            },
            Err(read_error) => match read_error {
                ReadError::ParseErr(e) => Err(ResponseError::from(e)),
                ReadError::IOError(e) if e.kind() == ErrorKind::ConnectionAborted => {
                    dlog!("One client disconnected successfully");
                    break;
                }
                ReadError::IOError(e) => {
                    elog!("Error reading from socket '{e}', disconnecting...");
                    break;
                }
            },
        };

        let response = match response {
            Err(e) => Response::Err(e),
            Ok(k) => Response::Ok(k),
        };

        dlog!("Writing a Response");
        if let Err(e) = pipe.write_response(response) {
            elog!("Error writing to socket '{e}', disconnecting...");
            break;
        }
    }
}

/// Listens for incoming connections and handles them
pub fn listen() -> ! {
    let addr = opal_abi::CONNECT_ABSTRACT_ADDR;
    let mut listener_builder = UnixListenerBuilder::from_abstract_path(addr).unwrap();
    listener_builder
        .set_type(SockKind::SeqPacket)
        .set_backlog(usize::MAX);

    let listener = listener_builder.bind().expect("Failed to bind a listener");
    log!("WM Listening at {}", addr);

    spawn_hello();

    loop {
        let connection = listener
            .accept()
            .expect("Failed to Accept a pending connection");

        // TODO: Implement something similar to poll
        std::thread::spawn(|| handle_connect(connection));
    }
}
