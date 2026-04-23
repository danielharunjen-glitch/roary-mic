//! Cross-platform screen capture for AI-mode context.
//!
//! Uses `xcap` to grab the primary display, optionally downscales to keep LLM
//! image tokens bounded, and encodes to PNG.

use anyhow::{anyhow, Context, Result};
use image::{imageops::FilterType, DynamicImage, ImageBuffer, Rgba};
use std::io::Cursor;
use xcap::Monitor;

const MAX_DIMENSION: u32 = 1280;

/// Capture the primary display, downscale if any dimension exceeds
/// [`MAX_DIMENSION`], and return PNG-encoded bytes.
pub fn capture_primary_display_png() -> Result<Vec<u8>> {
    let monitors = Monitor::all().map_err(|e| anyhow!("Failed to enumerate monitors: {}", e))?;

    // Prefer the OS-designated primary monitor; fall back to the first one.
    let monitor = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())
        .ok_or_else(|| anyhow!("No monitors available for screen capture"))?;

    let rgba = monitor
        .capture_image()
        .map_err(|e| anyhow!("Screen capture failed: {}", e))?;

    let (w, h) = (rgba.width(), rgba.height());
    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(w, h, rgba.into_raw())
        .ok_or_else(|| anyhow!("Screen capture produced invalid buffer ({}x{})", w, h))?;
    let mut img = DynamicImage::ImageRgba8(buffer);

    if w > MAX_DIMENSION || h > MAX_DIMENSION {
        let scale = (MAX_DIMENSION as f32 / w.max(h) as f32).min(1.0);
        let new_w = (w as f32 * scale).round().max(1.0) as u32;
        let new_h = (h as f32 * scale).round().max(1.0) as u32;
        img = img.resize(new_w, new_h, FilterType::Triangle);
    }

    let mut out = Vec::with_capacity(256 * 1024);
    img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .context("encoding screenshot PNG")?;
    Ok(out)
}
