use bincode::{Decode, Encode};
use bitflags::bitflags;

/// When the mouse cursor enters a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(C)]
pub struct MouseEnterEvent {
    /// The x-coordinate of the mouse cursor, relative to the window.
    pos_x: u32,
    /// The y-coordinate of the mouse cursor, relative to the window.
    pos_y: u32,
}

impl MouseEnterEvent {
    /// Creates a new `MouseEnterEvent`.
    pub fn new(pos_x: u32, pos_y: u32) -> Self {
        Self { pos_x, pos_y }
    }

    /// Returns the x-coordinate of the mouse cursor, relative to the window.
    pub const fn x(&self) -> u32 {
        self.pos_x
    }

    /// Returns the y-coordinate of the mouse cursor, relative to the window.
    pub const fn y(&self) -> u32 {
        self.pos_y
    }
}

/// When the mouse cursor leaves a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(C)]
pub struct MouseLeaveEvent;

impl MouseLeaveEvent {
    /// Creates a new `MouseLeaveEvent`.
    pub fn new() -> Self {
        Self
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HeldMouseButtons: u8 {
        const LEFT = 1 << 0;
        const MIDDLE = 1 << 1;
        const RIGHT = 1 << 2;
    }
}

impl Encode for HeldMouseButtons {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        u8::encode(&self.bits(), encoder)
    }
}

impl<Context> Decode<Context> for HeldMouseButtons {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        u8::decode(decoder).map(|bits| HeldMouseButtons::from_bits_retain(bits))
    }
}

bincode::impl_borrow_decode!(HeldMouseButtons);

/// When the mouse cursor moves within a window or a change to it's buttons occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(C)]
pub struct MouseChangeEvent {
    /// Whether or not the buttons has changed.
    buttons_changed: bool,
    /// The buttons that are currently held down.
    held_buttons: HeldMouseButtons,
    __: u16,
    /// The x-coordinate of the mouse cursor, relative to the window.
    pos_x: u32,
    /// The y-coordinate of the mouse cursor, relative to the window.
    pos_y: u32,
}

impl MouseChangeEvent {
    /// Creates a new `MouseChangeEvent`.
    pub fn new(
        buttons_changed: bool,
        held_buttons: HeldMouseButtons,
        pos_x: u32,
        pos_y: u32,
    ) -> Self {
        Self {
            buttons_changed,
            held_buttons,
            __: 0,
            pos_x,
            pos_y,
        }
    }

    /// Returns whether or not the buttons have changed.
    pub const fn buttons_changed(&self) -> bool {
        self.buttons_changed
    }

    /// Returns the buttons that are currently held down.
    pub const fn held_buttons(&self) -> HeldMouseButtons {
        self.held_buttons
    }

    /// Returns the change in buttons that occurred if the buttons have changed otherwise None.
    pub const fn buttons_change(&self) -> Option<HeldMouseButtons> {
        if self.buttons_changed {
            Some(self.held_buttons)
        } else {
            None
        }
    }

    /// Returns the x-coordinate of the mouse cursor, relative to the window.
    pub const fn x(&self) -> u32 {
        self.pos_x
    }

    /// Returns the y-coordinate of the mouse cursor, relative to the window.
    pub const fn y(&self) -> u32 {
        self.pos_y
    }
}

/// Represents an event that occurred on a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(u32)]
pub enum Event {
    MouseChange(MouseChangeEvent),
    MouseLeave(MouseLeaveEvent),
    MouseEnter(MouseEnterEvent),
    WindowFocused,
}
