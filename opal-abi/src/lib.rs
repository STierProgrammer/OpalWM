//! Defines layout of structures to communicate with the OpalWM

/// The abstract socket address to use to connect with the OpalWM
pub const CONNECT_ABSTRACT_ADDR: &str = "opal_wm::connect";

/// The communication protocol, contains he layout of packets
/// that can be sent to and from the WM
pub mod com;
