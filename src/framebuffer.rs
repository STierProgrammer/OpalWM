use std::{
    ptr::NonNull,
    sync::{LazyLock, Mutex, MutexGuard},
};

use safa_api::syscalls::types::Ri;
use std::fs::OpenOptions;
use std::os::safaos::AsRawResource;
use std::os::safaos::IoUtils;
use std::usize;

use safa_api::abi::mem::MemMapFlags;

use crate::dlog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a single pixel
#[repr(transparent)]
pub struct Pixel(u32);

impl Pixel {
    /// Construct a Pixel from an RGB Color
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::from_hex(((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    }
    /// Construct a Pixel from a hex RGB Color
    pub const fn from_hex(rgb: u32) -> Self {
        Self(rgb & 0xFFFFFF)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
/// A struct represinting information about the virtual framebuffer
pub struct FramebufferDevInfo {
    pub width: usize,
    pub height: usize,
    /// Bits per pixel, for now the virtual framebuffer always have 32bits per pixel
    bpp: usize,
    /// Whether or not each pixel is encoded as BGR and not RGB (always false for now)
    bgr: bool,
}

const CMD_RECEIVE_FB_INFO: u16 = 1;
const CMD_SYNC_PIXELS: u16 = 2;

/// A framebuffer
pub struct Framebuffer {
    width: usize,
    height: usize,
    pixels: &'static mut [Pixel],
    mmap_ri: Ri,
}

static FRAMEBUFFER_MEMMAP: LazyLock<(FramebufferDevInfo, Ri, (usize, usize))> =
    LazyLock::new(|| {
        let fb_file = OpenOptions::new()
            .write(true)
            .open("dev:/fb")
            .expect("failed to open the framebuffer");
        // First we want to receive the framebuffer info
        let mut fb_info: FramebufferDevInfo = unsafe { core::mem::zeroed() };
        fb_file
            .send_command(CMD_RECEIVE_FB_INFO, &raw mut fb_info as usize as u64)
            .expect("Failed to receive information about the framebuffer");

        assert!(fb_info.bpp == size_of::<u32>() * 8);
        assert!(!fb_info.bgr);

        dlog!("Got Framebuffer: {fb_info:#?}");
        let pixels_required = fb_info.height * fb_info.width;
        let bytes_required = (fb_info.bpp / 8) * pixels_required;

        // The Mapping should live as long as the Process
        let (fb_ri, bytes) = safa_api::syscalls::mem::map(
            core::ptr::null(),
            bytes_required / 4096,
            0,
            Some(fb_file.as_raw_resource()),
            None,
            MemMapFlags::WRITE,
        )
        .expect("Failed to SysMemMap the Framebuffer");

        (
            fb_info,
            fb_ri,
            (bytes.as_ptr() as *mut u8 as usize, bytes.len()),
        )
    });

/// Information about the framebuffer
pub static FB_INFO: LazyLock<FramebufferDevInfo> = LazyLock::new(|| {
    let (dev, _, _) = &*FRAMEBUFFER_MEMMAP;
    *dev
});

static FRAMEBUFFER: LazyLock<Mutex<Framebuffer>> = LazyLock::new(|| {
    let (dev, mmap_ri, (pixels_bytes_addr, bytes_len)) = &*FRAMEBUFFER_MEMMAP;
    let pixels_count = bytes_len / size_of::<Pixel>();
    let pixels =
        unsafe { std::slice::from_raw_parts_mut(*pixels_bytes_addr as *mut Pixel, pixels_count) };
    Mutex::new(Framebuffer {
        pixels,
        mmap_ri: *mmap_ri,
        width: dev.width,
        height: dev.height,
    })
});

impl Framebuffer {
    /// Draws a rectangle with the given pixels
    pub fn draw_rect(
        &mut self,
        off_x: usize,
        off_y: usize,
        width: usize,
        height: usize,
        pixels: &[Pixel],
    ) {
        for row in 0..height {
            let target_row_index = off_x + ((off_y + row) * self.width);
            let src_row_index = row * width;

            if target_row_index + width >= self.pixels.len() {
                return;
            }

            let target_pixels = &mut self.pixels[target_row_index..target_row_index + width];
            let src_pixels = &pixels[src_row_index..src_row_index + width];

            target_pixels.copy_from_slice(src_pixels);
        }
    }

    /// Draws a rectangle filled with a pixel `pixel`
    pub fn draw_rect_filled_with(
        &mut self,
        off_x: usize,
        off_y: usize,
        width: usize,
        height: usize,
        pixel: Pixel,
    ) {
        for row in 0..height {
            let row_index = off_x + ((off_y + row) * self.width);
            let pixels = &mut self.pixels[row_index..row_index + width];
            pixels.fill(pixel);
        }
    }

    /// Syncs the full framebuffer double buffer to the real buffer
    pub fn sync_pixels_full(&self) {
        self.sync_pixels_rect(0, 0, self.width, self.height);
    }

    /// Syncs a rectangle to the framebuffer
    pub fn sync_pixels_rect(&self, off_x: usize, off_y: usize, width: usize, height: usize) {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        struct SyncRect {
            off_x: usize,
            off_y: usize,
            width: usize,
            height: usize,
        }

        let rect = SyncRect {
            off_x,
            off_y,
            width,
            height,
        };

        safa_api::syscalls::io::io_command(
            self.mmap_ri,
            CMD_SYNC_PIXELS,
            (&raw const rect) as usize as u64,
        )
        .expect("Failed to Sync framebuffer")
    }
}

/// Returns a lock on the framebuffer interface
pub fn framebuffer() -> MutexGuard<'static, Framebuffer> {
    FRAMEBUFFER
        .lock()
        .expect("Failed to acquire lock on framebuffer")
}

/// Clears the screen
pub fn clear() {
    let mut fb = FRAMEBUFFER
        .lock()
        .expect("Failed to hold lock on framebuffer");
    fb.draw_rect_filled_with(0, 0, FB_INFO.width, FB_INFO.height, Pixel::from_hex(0));
    fb.sync_pixels_full();
    dlog!("Cleared screen");
}
