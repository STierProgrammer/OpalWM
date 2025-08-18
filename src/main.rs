use crate::bmp::BMPImage;
use crate::com::listener;
use crate::framebuffer::Pixel;
use crate::logging::disable_terminal_logging;
use crate::window::{WINDOWS, Window};

/// Set to true if you want really verbose slow information
///
/// TODO: make this a cmd line arg or perhaps a feature
const REALLY_VERBOSE: bool = false;

mod bmp;
mod com;
mod framebuffer;
mod logging;
mod mice;
mod window;

fn main() {
    log!("WM Starting");
    disable_terminal_logging();
    framebuffer::clear();
    {
        let mut w = WINDOWS.lock().expect("failed to get lock on windows");
        w.add_window(Window::new_filled_with(
            213,
            442,
            200,
            200,
            Pixel::from_rgba(0, 0xFF, 0, 0xFF),
        ));
        w.add_window(Window::new_filled_with(
            270,
            400,
            200,
            200,
            Pixel::from_rgba(0xFF, 0, 0, 0xFF / 2),
        ));
        w.damage_redraw();
    }
    std::thread::spawn(mice::mice_poll);
    listener::listen()
}
