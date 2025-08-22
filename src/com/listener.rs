use std::{
    io::ErrorKind,
    process::{Command, Stdio},
    sync::Arc,
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
    window::{self, WINDOWS, Window, WindowKind},
    wlog,
};

fn spawn_hello() {
    if let Err(err) = Command::new("sys:/bin/hello_world")
        .stdout(Stdio::from(logging::console_clone()))
        .stderr(Stdio::from(logging::console_clone()))
        .stdin(Stdio::from(logging::console_clone()))
        .spawn()
    {
        elog!("Failed to spawn hello_world process: {}", err);
    }
}

fn handle_connect(connection: UnixSockConnection) {
    dlog!("Handling a new connection");

    let mut window_ids = Vec::with_capacity(1);

    let pipe = Arc::new(ClientComPipe::new(connection));
    // No one else is going to be receiving requests and therefore we can take ownership of the receiver
    let mut receiver = pipe.receiver();

    loop {
        dlog!("Waiting for a Request");

        let response = match receiver.receive_request() {
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
                    )
                    .with_com_pipe(pipe.clone());

                    let shm_key = *window.shm_key();
                    window::add_window(window, WindowKind::Normal)
                        .map(|id| {
                            dlog!("Added Window {id}, with the SHM Key {shm_key} for a client");
                            window_ids.push(id);
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
        if let Err(e) = pipe.sender().send_response(response) {
            elog!("Error writing to socket '{e}', disconnecting...");
            break;
        }
    }

    // cleanup windows
    {
        let mut windows = WINDOWS
            .lock()
            .expect("Failed to acquire lock on Windows when cleaning up after disconnecting");
        for id in window_ids {
            if let Err(()) = windows.remove_window(id) {
                wlog!("Failed to remove window {id}");
            }
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
