use std::io::{self, Read, Write};

use opal_abi::com::{
    MAX_PACKET_SIZE,
    request::{RawRequest, Request, RequestParseErr},
    response::{RawResponse, Response},
};
use safa_api::sockets::UnixSockConnection;
use thiserror::Error;

pub mod listener;

/// A Wrapper over a bi-directonal communication pipe, that can send data to and from the client
///
/// This pipe works with the assumption that a single writer which is also a reader will happen to use it,
/// And therefore isn't wrapped in any locks
pub struct ClientComPipe(UnixSockConnection);

/// An Error that happened during reading a request from a Client
#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Failed to parse Client's Request {0}")]
    ParseErr(#[from] RequestParseErr),
    #[error("Error while reading from a socket: {0}")]
    IOError(#[from] io::Error),
}

impl ClientComPipe {
    pub const fn new(inner: UnixSockConnection) -> Self {
        Self(inner)
    }

    /// Reads 1 Request from the Client
    pub fn read_request(&self) -> Result<Request, ReadError> {
        let mut buf = [0u8; MAX_PACKET_SIZE];
        let len = (&mut &*self).read(&mut buf)?;
        let request = &buf[..len];
        Ok(RawRequest::try_from_bytes(request)?.into_request())
    }

    /// Writes 1 Response to the client's last request
    pub fn write_response(&self, response: Response) -> io::Result<()> {
        let raw: RawResponse = response.into();

        let bytes = raw.into_bytes();
        let len = (&mut &*self).write(bytes)?;
        debug_assert_eq!(len, bytes.len());
        Ok(())
    }
}

impl<'a> Read for &'a ClientComPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut &self.0, buf)
    }
}

impl<'a> Write for &'a ClientComPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut &self.0, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        (&mut &self.0).flush()
    }
}
