use std::{
    fs::File,
    io::{BufReader, Read},
};

use safa_api::abi::input::{MiceEvent, MouseEventKind};

use crate::{
    dlog,
    framebuffer::Pixel,
    window::{WINDOWS, Window},
};

/// Polls the mouse device for incoming events and handles them
pub fn mice_poll() -> ! {
    let file = File::open("dev:/inmice").expect("Failed to open the Mouse Device");
    let mut reader = BufReader::with_capacity(size_of::<MiceEvent>() * 1, file);
    let win = {
        let mut windows = WINDOWS.lock().expect("failed to get lock on windows");
        windows.add_window(Window::new_filled_with(
            0,
            0,
            16,
            16,
            Pixel::from_rgb(0xFF, 0, 0),
        ))
    };

    let mice_pixels = 16 * 16;

    loop {
        let mut event_bytes = [0u8; size_of::<MiceEvent>()];
        let len = reader
            .read(&mut event_bytes)
            .expect("Failed to read an event");

        if len == 0 {
            continue;
        }

        assert_eq!(len, size_of::<MiceEvent>());

        let event: MiceEvent = unsafe { core::mem::transmute(event_bytes) };

        match event.kind {
            MouseEventKind::Change => {
                let x_change = event.x_rel_change;
                let y_change = -event.y_rel_change;
                if x_change == 0 && y_change == 0 {
                    continue;
                }

                // dlog!("event {event:#?}");
                let mut windows = WINDOWS.lock().expect("failed to get lock on windows");

                windows.change_cord(win, x_change, y_change);
                windows.damage_redraw();
            }
            MouseEventKind::Null => unreachable!(),
        }
    }
}
