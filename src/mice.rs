use std::{
    fs::File,
    io::{BufReader, Read},
};

use safa_api::abi::input::{MiceEvent, MouseEventKind};

use crate::{
    bmp::BMPImage,
    window::{WINDOWS, Window},
};

const CURSOR_BYTES: &[u8] = include_bytes!("../assets/beta-cursor.bmp");

/// Polls the mouse device for incoming events and handles them
pub fn mice_poll() -> ! {
    let cursor_bmp = BMPImage::from_slice(CURSOR_BYTES).expect("Failed to parse cursor.bmp");

    let file = File::open("dev:/inmice").expect("Failed to open the Mouse Device");
    let mut reader = BufReader::with_capacity(size_of::<MiceEvent>() * 1, file);
    let win = {
        let mut windows = WINDOWS.lock().expect("failed to get lock on windows");
        windows.add_window(Window::new_from_bmp(0, 0, cursor_bmp))
    };

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
