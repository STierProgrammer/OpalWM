use std::{
    fs::File,
    io::{BufReader, Read},
};

use safa_api::abi::input::{MiceBtnStatus, MiceEvent, MouseEventKind};

use crate::{
    bmp::BMPImage,
    window::{WINDOWS, Window, WindowKind},
};

const CURSOR_BYTES: &[u8] = include_bytes!("../assets/epic-cursor.bmp");

/// Polls the mouse device for incoming events and handles them
pub fn mice_poll() -> ! {
    let cursor_bmp = BMPImage::from_slice(CURSOR_BYTES).expect("Failed to parse cursor.bmp");
    let cursor_width = cursor_bmp.width();
    let cursor_height = cursor_bmp.height();

    let file = File::open("dev:/inmice").expect("Failed to open the Mouse Device");
    let mut reader = BufReader::with_capacity(size_of::<MiceEvent>() * 1, file);
    let win = {
        let mut windows = WINDOWS.lock().expect("failed to get lock on windows");
        windows
            .add_window(Window::new_from_bmp(0, 0, cursor_bmp), WindowKind::Overlay)
            .expect("Failed to add the Mouse cursor's window")
    };

    let (mut x, mut y) = (0, 0);
    let mut button_left_was_pressed: bool = false;
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
                let x_change = (event.x_rel_change) as i32;
                let y_change = (-event.y_rel_change) as i32;

                let mut windows = WINDOWS.lock().expect("failed to get lock on windows");
                if !(x_change == 0 && y_change == 0) {
                    let (new_x, new_y) = windows.add_cord(win, x_change, y_change).unwrap();
                    x = new_x;
                    y = new_y;
                }

                // dlog!("event {event:#?}");

                if button_left_was_pressed && let Some(focused_id) = windows.focused_window() {
                    windows.add_cord(focused_id, x_change, y_change);
                }

                if event.buttons_status.contains(MiceBtnStatus::BTN_LEFT)
                    && !button_left_was_pressed
                {
                    windows.focus_window_in_contact(x, y, cursor_width, cursor_height);
                }

                windows.damage_redraw();
                button_left_was_pressed = event.buttons_status.contains(MiceBtnStatus::BTN_LEFT);
            }
            MouseEventKind::Null => unreachable!(),
        }
    }
}
