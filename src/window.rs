use std::{iter::Sum, ops::Add, sync::Mutex};

use crate::{
    REALLY_VERBOSE,
    bmp::BMPImage,
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
    /// Creates a new Window from a given BMP Image
    pub fn new_from_bmp(pos_x: usize, pos_y: usize, image: BMPImage) -> Window {
        Self::new_from_pixels(pos_x, pos_y, image.width(), image.height(), image.pixels())
    }

    /// Creates a new Window and fills it with `fill_pixels`
    pub fn new_from_pixels(
        pos_x: usize,
        pos_y: usize,
        width: usize,
        height: usize,
        fill_pixels: impl ExactSizeIterator + Iterator<Item = Pixel>,
    ) -> Window {
        let mut pixels = vec![Pixel::from_hex(0); width * height];

        assert_eq!(
            pixels.len(),
            fill_pixels.len(),
            "The pixels to fill with must have a length of width*height"
        );

        let fill_pixels = fill_pixels.enumerate();
        for (i, pi) in fill_pixels {
            pixels[i] = pi;
        }

        let pixels = pixels.into_boxed_slice();

        Window {
            pos_x,
            pos_y,
            width,
            height,
            pixels,
        }
    }

    /// Creates a new Window and fills it repeatedly with a given `pixel`
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

    /// Draws the whole window without syncing the results to the real framebuffer.
    ///
    /// [`fb.sync_pixels_rect`] must be called afterwards on the area the window is in.
    fn draw(&self, fb: &mut Framebuffer) {
        fb.draw_rect(
            self.pos_x,
            self.pos_y,
            self.width,
            self.height,
            &self.pixels,
        );
    }

    /// Draws the window from intersection point without syncing the results to the real framebuffer.
    ///
    /// [`fb.sync_pixels_rect`] must be called afterwards on the area the window is in.
    fn draw_at(&self, fb: &mut Framebuffer, point: IntersectionPoint) {
        let (top_x_within, top_y_within) = point.top_left_within;
        let width = point.width();
        let height = point.height();

        let pixels = &self.pixels;
        let pixels_width = self.width;
        let pixels_height = self.height;

        if width == pixels_width && height == pixels_height {
            return self.draw(fb);
        }

        // The offset within the FB is the offset of self + the point
        let off_x = self.pos_x + top_x_within;
        let off_y = self.pos_y + top_y_within;

        // We want to draw pixels that `point` cover only
        fb.draw_rect_within(
            off_x,
            off_y,
            width,
            height,
            pixels,
            pixels_width,
            pixels_height,
            top_x_within,
            top_y_within,
        );
    }

    /// Returns the damage a window may have caused on the framebuffer, if it's position or dimensions changed
    /// There is 2 damages: The damage before the operation, The damage after the operation
    fn damage(&self) -> DamageRegion {
        DamageRegion {
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntersectionPoint {
    top_left_within: (usize, usize),
    bottom_right_within: (usize, usize),
}

impl IntersectionPoint {
    pub const fn none() -> Self {
        Self {
            top_left_within: (0, 0),
            bottom_right_within: (0, 0),
        }
    }

    pub const fn width(&self) -> usize {
        let (top_x, _) = self.top_left_within;
        let (bott_x, _) = self.bottom_right_within;
        bott_x - top_x
    }

    pub const fn height(&self) -> usize {
        let (_, top_y) = self.top_left_within;
        let (_, bott_y) = self.bottom_right_within;
        bott_y - top_y
    }
}

impl Add<IntersectionPoint> for IntersectionPoint {
    type Output = IntersectionPoint;
    fn add(self, rhs: IntersectionPoint) -> Self::Output {
        let (s_top_x, s_top_y) = self.top_left_within;
        let (o_top_x, o_top_y) = rhs.top_left_within;
        let (s_bott_x, s_bott_y) = self.bottom_right_within;
        let (o_bott_x, o_bott_y) = rhs.bottom_right_within;
        Self {
            top_left_within: (s_top_x.min(o_top_x), s_top_y.min(o_top_y)),
            bottom_right_within: (s_bott_x.max(o_bott_x), s_bott_y.max(o_bott_y)),
        }
    }
}

impl Sum for IntersectionPoint {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut results = IntersectionPoint::none();

        for i in iter {
            if results == IntersectionPoint::none() {
                results = i;
            } else {
                results = results + i;
            }
        }

        results
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
    /// Checks if self overlaps with `win` returning the point which is covered from the window
    pub fn overlaps_with(&self, win: &Window) -> Option<IntersectionPoint> {
        let d_x0 = self.pos_x;
        let d_x1 = self.pos_x + self.width;
        let d_y0 = self.pos_y;
        let d_y1 = self.pos_y + self.height;

        let w_x0 = win.pos_x;
        let w_x1 = win.pos_x + win.width;
        let w_y0 = win.pos_y;
        let w_y1 = win.pos_y + win.height;

        if (d_x0 < w_x1 && d_x1 > w_x0) && (d_y0 < w_y1 && d_y1 > w_y0) {
            let i_x0 = d_x0.max(w_x0) - w_x0;
            let i_x1 = d_x1.min(w_x1) - w_x0;
            let i_y0 = d_y0.max(w_y0) - w_y0;
            let i_y1 = d_y1.min(w_y1) - w_y0;

            Some(IntersectionPoint {
                top_left_within: (i_x0, i_y0),
                bottom_right_within: (i_x1, i_y1),
            })
        } else {
            None
        }
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
            let intersection: IntersectionPoint =
                damage.iter().filter_map(|d| d.overlaps_with(win)).sum();

            if intersection != IntersectionPoint::none() {
                win.draw_at(&mut fb, intersection);
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

        if win.pos_x == damage0.pos_x && win.pos_y == damage0.pos_y {
            return;
        }

        let damage1 = win.damage();

        if REALLY_VERBOSE {
            dlog!(
                "window changed from x: {}, y: {} to x: {}, y: {} as per: {x}, {y}",
                damage0.pos_x,
                damage0.pos_y,
                damage1.pos_x,
                damage1.pos_y
            );
        }
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
