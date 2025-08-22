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
    const CORNER_RADIUS: u32 = 8;
    const BORDER_COLOR: Pixel = Pixel::from_rgba(0xFD, 0xB0, 0xC0, 0xFF);
    const BG_COLOR: Pixel = Pixel::from_rgba(0, 0, 0, 0x80);
    const TITLE_HEIGHT: u32 = 20;

    pub fn new(width: u32, height: u32) -> Self {
        let real_width = width + Self::CORNER_RADIUS;
        let real_height = height + Self::TITLE_HEIGHT;
        let window_x = Self::CORNER_RADIUS / 2;
        let window_y = Self::TITLE_HEIGHT;

        let mut win = Window::create(0, 0, real_width, real_height);

        win.draw_round_rect(
            0,
            0,
            real_width,
            real_height,
            Self::CORNER_RADIUS,
            |is_border, line_num| {
                if is_border {
                    Self::BORDER_COLOR
                } else {
                    if line_num < Self::TITLE_HEIGHT {
                        Self::BORDER_COLOR
                    } else {
                        Self::BG_COLOR
                    }
                }
            },
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
