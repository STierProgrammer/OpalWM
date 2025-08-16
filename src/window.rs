use std::sync::Mutex;

use crate::{
    dlog,
    framebuffer::{self, FB_INFO, Framebuffer, Pixel},
};

// a Rectangle
pub struct Window {
    //
    pos_x: usize,
    pos_y: usize,
    //
    width: usize,
    height: usize,
    //
    pixels: Box<[Pixel]>,
}

impl Window {
    pub fn new_filled_with(
        pos_x: usize,
        pos_y: usize,
        width: usize,
        height: usize,
        pixel: Pixel,
    ) -> Self {
        let pixels = vec![pixel; width * height];
        let pixels = pixels.into_boxed_slice();

        Window {
            pos_x,
            pos_y,
            width,
            height,
            pixels,
        }
    }
    pub fn draw(&self, fb: &mut Framebuffer) {
        fb.draw_rect(
            self.pos_x,
            self.pos_y,
            self.width,
            self.height,
            &self.pixels,
        );
    }

    pub fn damage(&self) -> DamageRegion {
        DamageRegion {
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DamageRegion {
    pos_x: usize,
    pos_y: usize,
    width: usize,
    height: usize,
}

impl DamageRegion {
    pub const fn overlaps_with(&self, win: &Window) -> bool {
        let d_x0 = self.pos_x;
        let d_x1 = self.pos_x + self.width;
        let d_y0 = self.pos_y;
        let d_y1 = self.pos_y + self.height;

        let w_x0 = win.pos_x;
        let w_x1 = win.pos_x + win.width;
        let w_y0 = win.pos_y;
        let w_y1 = win.pos_y + win.height;

        (d_x0 < w_x1 && d_x1 > w_x0) && (d_y0 < w_y1 && d_y1 > w_y0)
    }
}

pub struct Windows {
    windows: Vec<Window>,
    damage_regions: Vec<DamageRegion>,
}

impl Windows {
    pub const fn new() -> Self {
        Self {
            damage_regions: Vec::new(),
            windows: Vec::new(),
        }
    }

    pub fn damage_redraw(&mut self) {
        let mut fb = framebuffer::framebuffer();

        let damage = core::mem::take(&mut self.damage_regions);

        for region in &damage {
            // Clear the damaged region
            fb.draw_rect_filled_with(
                region.pos_x,
                region.pos_y,
                region.width,
                region.height,
                Pixel::from_hex(0x0),
            );
        }

        for win in &self.windows {
            if damage.iter().any(|r| r.overlaps_with(win)) {
                win.draw(&mut fb);
            }
        }

        for r in damage {
            fb.sync_pixels_rect(r.pos_x, r.pos_y, r.width, r.height);
        }
    }

    pub fn change_cord(&mut self, win_id: usize, x: i16, y: i16) {
        if x == 0 && y == 0 {
            return;
        }

        let win = &mut self.windows[win_id];
        let damage0 = win.damage();

        let max_x = FB_INFO.width;
        let max_y = FB_INFO.height;

        win.pos_x = std::cmp::min(
            win.pos_x.saturating_add_signed(x as isize),
            max_x - win.width,
        );
        win.pos_y = std::cmp::min(
            win.pos_y.saturating_add_signed(y as isize),
            max_y - win.height,
        );

        let damage1 = win.damage();

        dlog!(
            "window changed from x: {}, y: {} to x: {}, y: {} as per: {x}, {y}",
            damage0.pos_x,
            damage0.pos_y,
            damage1.pos_x,
            damage1.pos_y
        );
        self.damage_regions.push(damage0);
        self.damage_regions.push(damage1);
    }

    pub fn add_window(&mut self, window: Window) -> usize {
        let id = self.windows.len();
        let damage = window.damage();
        self.windows.push(window);
        self.damage_regions.push(damage);
        id
    }
}

pub static WINDOWS: Mutex<Windows> = Mutex::new(Windows::new());
