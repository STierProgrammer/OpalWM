use std::fs::OpenOptions;
use std::os::safaos::AsRawResource;
use std::os::safaos::IoUtils;
use std::usize;

use safa_api::abi::mem::MemMapFlags;

use crate::com::listener;
use crate::logging::disable_terminal_logging;
mod com;
mod logging;

#[derive(Debug)]
#[repr(C)]
/// A struct represinting information about the virtual framebuffer
pub struct FramebufferDevInfo {
    width: usize,
    height: usize,
    /// Bits per pixel, for now the virtual framebuffer always have 32bits per pixel
    bpp: usize,
    /// Whether or not each pixel is encoded as BGR and not RGB (always false for now)
    bgr: bool,
}

const CMD_RECEIVE_FB_INFO: u16 = 1;

fn main() {
    log!("WM Starting");

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

    // So this is static because the mapping lives as long as the process
    // As long as we don't destroy it
    let pixels: &'static mut [u32] =
        unsafe { std::slice::from_raw_parts_mut(bytes.as_ptr() as *mut u32, pixels_required) };
    pixels.fill(0xFFFFFFFF);

    log!(
        "Cleared screen, {}KiB worth of pixels",
        (pixels.len() * 4) / 1024
    );
    disable_terminal_logging();

    safa_api::syscalls::io::sync(fb_ri).expect("failed to sync the FB");
    listener::listen()
}
