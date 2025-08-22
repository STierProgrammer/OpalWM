mod canvas;

pub use libopal;
use libopal::window::{Pixel, Window};

use crate::canvas::DrawingCanvas;

struct RootContainer {
    root: Window,
    width: u32,
    height: u32,
    window_x: u32,
    window_y: u32,
}

impl RootContainer {
    const CORNER_RADIUS: u32 = 25;
    const BORDER_COLOR: Pixel = Pixel::from_rgba(0xFF, 00, 0x7F, 0xF0);

    pub fn new(width: u32, height: u32) -> Self {
        let real_width = width + Self::CORNER_RADIUS;
        let real_height = height + Self::CORNER_RADIUS;
        let window_x = Self::CORNER_RADIUS / 2;
        let window_y = Self::CORNER_RADIUS / 2;

        let mut win = Window::create(0, 0, real_width, real_height);

        win.draw_round_rect(
            0,
            0,
            real_width,
            real_height,
            Self::CORNER_RADIUS,
            Self::BORDER_COLOR,
            Pixel::from_rgba(0x0, 0x0, 0x0, 0x80),
        );

        win.redraw(0, 0, real_width, real_height);
        Self {
            root: win,
            width,
            height,
            window_x,
            window_y,
        }
    }
}
pub struct Gem {
    root: RootContainer,
}

impl Gem {
    pub fn init(width: u32, height: u32) -> Self {
        libopal::init();
        let root_container = RootContainer::new(width, height);
        Self {
            root: root_container,
        }
    }
}
