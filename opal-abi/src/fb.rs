#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a single pixel
#[repr(C)]
pub struct Pixel {
    blue: u8,
    green: u8,
    red: u8,
    alpha: u8,
}

impl Pixel {
    /// Construct a Pixel from an RGBA Color
    pub const fn from_rgba(r: u8, g: u8, b: u8, alpha: u8) -> Self {
        Self {
            blue: b,
            green: g,
            red: r,
            alpha,
        }
    }
    /// Construct a Pixel from a hex RGB Color
    pub const fn from_hex(argb: u32) -> Self {
        unsafe { core::mem::transmute(argb) }
    }
}
