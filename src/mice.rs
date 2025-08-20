use std::{
    fs::File,
    io::{BufReader, Read},
};

use safa_api::abi::input::{MiceBtnStatus, MiceEvent, MouseEventKind};

use crate::{
    bmp::BMPImage,
    dlog,
    window::{WINDOWS, WinID, Window, WindowKind},
};

const CURSOR_BYTES: &[u8] = include_bytes!("../assets/epic-cursor.bmp");

/// The MiceCursor struct represents a mouse cursor on the screen, also handles mouse events.
pub struct MiceCursor {
    win_id: WinID,
    x: usize,
    y: usize,
    height: usize,
    width: usize,
    left_button_was_pressed: bool,
    reader: BufReader<File>,
}

impl MiceCursor {
    /// Creates a new MiceCursor instance
    pub fn create() -> Self {
        let cursor_bmp = BMPImage::from_slice(CURSOR_BYTES).expect("Failed to parse cursor.bmp");
        let cursor_width = cursor_bmp.width();
        let cursor_height = cursor_bmp.height();

        let file = File::open("dev:/inmice").expect("Failed to open the Mouse Device");
        let reader = BufReader::with_capacity(size_of::<MiceEvent>() * 1, file);
        let win = {
            let mut windows = WINDOWS.lock().expect("failed to get lock on windows");
            windows
                .add_window(Window::new_from_bmp(0, 0, cursor_bmp), WindowKind::Overlay)
                .expect("Failed to add the Mouse cursor's window")
        };

        dlog!("Added window {win} for cursor");
        Self {
            win_id: win,
            x: 0,
            y: 0,
            left_button_was_pressed: false,
            height: cursor_height,
            width: cursor_width,
            reader,
        }
    }

    /// Handles one mouse event if available
    pub fn handle_event(&mut self) {
        let mut event_bytes = [0u8; size_of::<MiceEvent>()];
        let len = self
            .reader
            .read(&mut event_bytes)
            .expect("Failed to read an event");

        if len == 0 {
            return;
        }

        assert_eq!(len, size_of::<MiceEvent>());

        let event: MiceEvent = unsafe { core::mem::transmute(event_bytes) };

        match event.kind {
            MouseEventKind::Change => {
                let x_change = (event.x_rel_change) as i32;
                let y_change = (-event.y_rel_change) as i32;

                let mut windows = WINDOWS.lock().expect("failed to get lock on windows");
                if !(x_change == 0 && y_change == 0) {
                    let (new_x, new_y) = windows.add_cord(self.win_id, x_change, y_change).unwrap();
                    self.x = new_x;
                    self.y = new_y;
                }

                // dlog!("event {event:#?}");

                if self.left_button_was_pressed
                    && let Some(focused_id) = windows.focused_window()
                {
                    windows.add_cord(focused_id, x_change, y_change);
                }

                if event.buttons_status.contains(MiceBtnStatus::BTN_LEFT)
                    && !self.left_button_was_pressed
                {
                    windows.focus_window_in_contact(self.x, self.y, self.width, self.height);
                }

                self.left_button_was_pressed =
                    event.buttons_status.contains(MiceBtnStatus::BTN_LEFT);
            }
            MouseEventKind::Null => unreachable!(),
        }
    }
}
