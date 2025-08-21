use std::{
    cell::UnsafeCell,
    io::{self, Read, Write},
    sync::{Mutex, MutexGuard},
};

use opal_abi::com::{
    packet::{MAX_PACKET_SIZE, PacketParseErr},
    request::Request,
    response::Response,
};
use safa_api::sockets::UnixSockConnection;
use thiserror::Error;

pub mod listener;

/// A lock guard for the Sender part of the [`ClientComPipe`]
pub struct ClientComSender<'a> {
    _guard: MutexGuard<'a, ()>,
    pipe: &'a ClientComPipe,
}

impl ClientComSender<'_> {
    /// Sends a response to the client, blocks until the response is sent.
    pub fn send_response(&mut self, response: Response) -> Result<(), io::Error> {
        let (bytes, len) = response.encode();
        let bytes = &bytes[..len];

        let len = self.write(bytes)?;
        debug_assert_eq!(len, bytes.len());
        Ok(())
    }
}

/// A lock guard for the Receiver part of the [`ClientComPipe`].
pub struct ClientComReceiver<'a> {
    _guard: MutexGuard<'a, ()>,
    pipe: &'a ClientComPipe,
}

impl ClientComReceiver<'_> {
    /// Receives a request from the client, blocks until the request is received.
    pub fn receive_request(&mut self) -> Result<Request, ReadError> {
        let mut buf = [0u8; MAX_PACKET_SIZE];
        let len = self.read(&mut buf)?;
        let request = &buf[..len];
        Ok(Request::decode(request)?)
    }
}

/// A Wrapper over a bi-directonal communication pipe, that can send data to and from the client.
///
/// This structure allows you to separate read and write operations on the client giving different locks for send and receive operations,
/// obviously this means that there is no guarantee that the client will receive the response in the same order as the request was sent, but allows to send events to the client.
pub struct ClientComPipe {
    sender_lock: Mutex<()>,
    receiver_lock: Mutex<()>,
    connection: UnsafeCell<UnixSockConnection>,
}

/// An Error that happened during reading a request from a Client
#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Failed to parse Client's Request {0}")]
    ParseErr(#[from] PacketParseErr),
    #[error("Error while reading from a socket: {0}")]
    IOError(#[from] io::Error),
}

impl ClientComPipe {
    pub const fn new(inner: UnixSockConnection) -> Self {
        Self {
            sender_lock: Mutex::new(()),
            receiver_lock: Mutex::new(()),
            connection: UnsafeCell::new(inner),
        }
    }

    /// Acquires lock on a sender that can be used to send responses to the client.
    pub fn sender<'a>(&'a self) -> ClientComSender<'a> {
        ClientComSender {
            _guard: self
                .sender_lock
                .lock()
                .expect("Failed to acquire lock on the sending side of a communication pipe"),
            pipe: self,
        }
    }

    /// Acquires lock on a receiver that can be used to receive requests from the client.
    pub fn receiver<'a>(&'a self) -> ClientComReceiver<'a> {
        ClientComReceiver {
            _guard: self
                .receiver_lock
                .lock()
                .expect("Failed to acquire lock on the receiving side of a communication pipe"),
            pipe: self,
        }
    }
}

impl<'a> Read for ClientComReceiver<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe { Read::read(&mut *self.pipe.connection.get(), buf) }
    }
}

impl<'a> Write for ClientComSender<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe { Write::write(&mut *self.pipe.connection.get(), buf) }
    }
    fn flush(&mut self) -> io::Result<()> {
        unsafe { &mut *self.pipe.connection.get() }.flush()
    }
}
