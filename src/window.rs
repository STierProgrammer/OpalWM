use std::{collections::HashMap, iter::Sum, ops::Add, sync::Mutex};

use indexmap::IndexSet;
use rustc_hash::{FxBuildHasher, FxHashMap};

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
    #[inline]
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

const MAX_WINDOW_ID: usize = 1024 /* TODO: more windows? */;
/// A window ID
pub type WinID = u16;

/// The type of the Window, defines the ordering which a Window may come over another, for example the cursor uses [`WindowKind::Overlay`]
#[derive(Debug, Clone, Copy)]
pub enum WindowKind {
    /// Always displayed above all other windows
    Overlay,
    /// Normal ordering
    Normal,
}

pub struct Windows {
    windows: FxHashMap<WinID, (Window, WindowKind)>,
    /// Windows that always come on top of other windows
    overlay_windows: IndexSet<WinID, FxBuildHasher>,
    /// The ordering of the windows in the Z Axis, the focused Window comes last
    normal_windows: IndexSet<WinID, FxBuildHasher>,

    /// A list of window IDs
    /// currently stored using a Bitmap and the max is 1024
    window_ids: [u128; 8],
    focused_window: Option<WinID>,

    damage_regions: Vec<DamageRegion>,
}

impl Windows {
    pub const fn new() -> Self {
        Self {
            overlay_windows: IndexSet::with_hasher(FxBuildHasher),
            normal_windows: IndexSet::with_hasher(FxBuildHasher),
            focused_window: None,

            damage_regions: Vec::new(),
            windows: HashMap::with_hasher(FxBuildHasher),
            window_ids: [0; 8],
        }
    }

    /// Allocates a new Window ID
    fn add_id(&mut self) -> Option<WinID> {
        for (row, byte) in self.window_ids.iter_mut().enumerate() {
            let width = size_of_val(byte) * 8;

            for col in 0..width {
                let bit = ((*byte >> col) & 1) == 1;
                if !bit {
                    *byte |= 1 << col;
                    return Some((col + (row * width)) as WinID);
                }
            }
        }

        None
    }

    /// Deallocate an existing Window ID
    /// returns true if successful, false if the ID is invalid
    fn remove_id(&mut self, id: WinID) -> bool {
        if id as usize >= MAX_WINDOW_ID {
            return false;
        }

        let width = size_of_val(&self.window_ids[0]);
        let row = (id / width as u16) as usize;
        let col = (id % width as u16) as usize;

        let byte = &mut self.window_ids[row];
        let bit = ((*byte >> col) & 1) == 1;
        let will_succeed = !bit;

        if will_succeed {
            *byte &= !(1 << col);
        }

        will_succeed
    }

    /// Redraw the damage caused by (and apply the results of) playing around with the windows using `self`
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

        // Fixes all the damages caused on a window if any
        macro_rules! fix_window {
            ($win: expr) => {{
                let win = $win;
                let intersection: IntersectionPoint =
                    damage.iter().filter_map(|d| d.overlaps_with(&win)).sum();

                if intersection != IntersectionPoint::none() {
                    win.draw_at(&mut fb, intersection);
                }
            }};
        }

        for win_id in &self.normal_windows {
            let (window, _) = self
                .windows
                .get_mut(win_id)
                .expect("Window wasn't removed from the Z-Ordering when it was removed");
            fix_window!(window);
        }

        // Overlay on top of other windows
        for win_id in &self.overlay_windows {
            let (window, _) = self
                .windows
                .get_mut(win_id)
                .expect("Overlay window wasn't removed from the Z-Ordering when it was removed");
            fix_window!(window);
        }

        for r in damage {
            fb.sync_pixels_rect(r.pos_x, r.pos_y, r.width, r.height);
        }
    }

    /// Adds `x` to window with the ID  `win_id` x position and `y` to the window with the ID `win_id`'s Y position
    ///
    /// Returns the new position if the Window ID exist
    pub fn add_cord(&mut self, win_id: WinID, x: i32, y: i32) -> Option<(usize, usize)> {
        let (win, _) = self.windows.get_mut(&win_id)?;

        /* The guarantee that this will be successful, is that we have a mutable reference on Self and that all access on the Window will be performed from Self */
        if x == 0 && y == 0 {
            return Some((win.pos_x, win.pos_y));
        }

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
            return Some((win.pos_x, win.pos_y));
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

        // Faster than extend_from_slice for some reason (I checked the code they aren't reserving the additional elements)
        self.damage_regions.reserve(2);
        self.damage_regions.push(damage0);
        self.damage_regions.push(damage1);
        Some((damage1.pos_x, damage1.pos_y))
    }

    /// Adds a window and organizes it depending on `kind` (see [`WindowKind`])
    pub fn add_window(&mut self, window: Window, kind: WindowKind) -> Option<WinID> {
        let damage = window.damage();

        let id = self.add_id()?;
        self.windows.insert(id, (window, kind));

        match kind {
            WindowKind::Normal => self.normal_windows.insert(id),
            WindowKind::Overlay => self.overlay_windows.insert(id),
        };

        self.damage_regions.push(damage);

        Some(id)
    }

    /// Makes the top most [`WindowKind::Normal`] Window that contacts,
    /// with the region at the position `pos_x`, `pos_y` with the width `width`
    /// and the height `height`, focused, returns the Window ID, or None if there are no normal windows in contact,
    /// If there are no normal windows on contact, a [`Self::focused_window`] call will return None after this.
    pub fn focus_window_in_contact(
        &mut self,
        pos_x: usize,
        pos_y: usize,
        width: usize,
        height: usize,
    ) -> Option<WinID> {
        let region = DamageRegion {
            pos_x,
            pos_y,
            width,
            height,
        };

        let win_id = self
            .normal_windows
            .iter()
            .rev()
            .find(|win_id| {
                let (win, _) = self.windows.get(win_id).expect(
                    "Window wasn't removed from the Z-ordering when it's ID was deallocated",
                );
                // TODO: this could be wrote better, perhaps?
                let overlaps = region.overlaps_with(win).is_some();
                if overlaps {
                    self.damage_regions.push(win.damage());
                }
                overlaps
            })
            .copied();

        if let Some(win_id) = win_id {
            self.normal_windows.shift_remove(&win_id);
            self.normal_windows.insert(win_id);
            self.focused_window = Some(win_id);
        } else {
            self.focused_window = None;
        }
        win_id
    }

    /// Returns the ID of the focused Window
    pub const fn focused_window(&self) -> Option<WinID> {
        self.focused_window
    }
}

pub static WINDOWS: Mutex<Windows> = Mutex::new(Windows::new());
