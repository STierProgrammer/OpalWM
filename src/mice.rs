use std::{
    fs::File,
    io::{BufReader, Read},
};

use opal_abi::com::response::event::{
    Event, HeldMouseButtons, MouseChangeEvent, MouseEnterEvent, MouseLeaveEvent,
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
    last_mouse_event: MiceEvent,
    current_window: Option<WinID>,
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
            height: cursor_height,
            width: cursor_width,
            last_mouse_event: MiceEvent {
                kind: MouseEventKind::Null,
                buttons_status: MiceBtnStatus::NO_BUTTONS,
                x_rel_change: 0,
                y_rel_change: 0,
            },
            current_window: None,
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
                let old_win_id = self.current_window;
                let left_button_was_pressed = self
                    .last_mouse_event
                    .buttons_status
                    .contains(MiceBtnStatus::BTN_LEFT);

                let x_change = (event.x_rel_change) as i32;
                let y_change = (-event.y_rel_change) as i32;

                let mut windows = WINDOWS.lock().expect("failed to get lock on windows");

                if !(x_change == 0 && y_change == 0) {
                    let (new_x, new_y) = windows.add_cord(self.win_id, x_change, y_change).unwrap();
                    self.x = new_x;
                    self.y = new_y;
                }

                let window_in_contact =
                    windows.window_in_contact(self.x, self.y, self.width, self.height);

                let left_button_is_pressed = event.buttons_status.contains(MiceBtnStatus::BTN_LEFT);
                if left_button_was_pressed
                    && let Some(focused_id) = windows.focused_window()
                    && left_button_is_pressed
                {
                    windows.add_cord(focused_id, x_change, y_change);
                }

                match window_in_contact {
                    Some((curr_id, contact_point)) => {
                        let mut mouse_enter = false;
                        let x = contact_point.x() as u32;
                        let y = contact_point.y() as u32;

                        if old_win_id.is_none_or(|old_id| old_id != curr_id) {
                            windows
                                .send_event(curr_id, Event::MouseEnter(MouseEnterEvent::new(x, y)))
                                .expect("Window removed before we could send an event to it");
                            mouse_enter = true;
                        }

                        if let Some(old_id) = old_win_id
                            && mouse_enter
                        {
                            /* It is ok the old window might be gone by now */
                            _ = windows
                                .send_event(old_id, Event::MouseLeave(MouseLeaveEvent::new()));
                        }

                        // FIXME: for some reason mouse release events are not being sent by the kernel driver.
                        if !mouse_enter {
                            let mut held_buttons = HeldMouseButtons::empty();

                            if left_button_is_pressed {
                                held_buttons.insert(HeldMouseButtons::LEFT);
                            }

                            if event.buttons_status.contains(MiceBtnStatus::BTN_MID) {
                                held_buttons.insert(HeldMouseButtons::MIDDLE);
                            }

                            if event.buttons_status.contains(MiceBtnStatus::BTN_RIGHT) {
                                held_buttons.insert(HeldMouseButtons::RIGHT);
                            }

                            let buttons_changed =
                                self.last_mouse_event.buttons_status != event.buttons_status;

                            let change_event =
                                MouseChangeEvent::new(buttons_changed, held_buttons, x, y);
                            windows.send_event(curr_id, Event::MouseChange(change_event)).expect("Current Window was removed before we could handle a mouse event");
                        }

                        if windows
                            .focused_window()
                            .is_none_or(|focus_id| focus_id != curr_id)
                            && left_button_is_pressed
                            && !left_button_was_pressed
                        {
                            windows.set_focused(curr_id);
                        }
                    }
                    None => {
                        if let Some(old_id) = old_win_id {
                            /* It is ok the old window might be gone by now */
                            _ = windows
                                .send_event(old_id, Event::MouseLeave(MouseLeaveEvent::new()));
                        }

                        if left_button_is_pressed && !left_button_was_pressed {
                            windows.unfocus_current();
                        }
                    }
                }

                self.last_mouse_event = event;
                self.current_window = window_in_contact.map(|(id, _)| id);
            }
            MouseEventKind::Null => unreachable!(),
        }
    }
}
