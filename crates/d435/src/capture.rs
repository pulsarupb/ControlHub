//! Capture and JPEG encoding adapted from Kinova-VLA/crates/webcams.
use crate::detect::Device;
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    utils::{CameraFormat, CameraIndex, RequestedFormat, RequestedFormatType},
};

pub fn open(device: &Device) -> Result<Camera, String> {
    let mut camera = Camera::new(
        CameraIndex::Index(device.index),
        RequestedFormat::with_formats(
            RequestedFormatType::Closest(CameraFormat::new_from(
                device.width,
                device.height,
                device.format,
                device.fps,
            )),
            &[device.format],
        ),
    )
    .map_err(|e| e.to_string())?;
    camera.open_stream().map_err(|e| e.to_string())?;
    Ok(camera)
}

pub fn encode(rgb: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut compress = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    compress.set_size(width as usize, height as usize);
    compress.set_quality(85.0);
    let mut output = compress
        .start_compress(Vec::new())
        .map_err(|e| e.to_string())?;
    output.write_scanlines(rgb).map_err(|e| e.to_string())?;
    output.finish().map_err(|e| e.to_string())
}

pub fn frame(camera: &mut Camera) -> Result<Vec<u8>, String> {
    let frame = camera.frame().map_err(|e| e.to_string())?;
    let rgb = frame
        .decode_image::<RgbFormat>()
        .map_err(|e| e.to_string())?;
    encode(rgb.as_raw(), rgb.width(), rgb.height())
}

pub fn crosshair(jpg: &[u8]) -> Result<Vec<u8>, String> {
    let mut rgb = image::load_from_memory(jpg)
        .map_err(|e| e.to_string())?
        .to_rgb8();
    let (w, h) = rgb.dimensions();
    let (x, y) = (w as i32 / 2, h as i32 / 2);
    for d in -32_i32..=32 {
        if d.abs() < 9 {
            continue;
        }
        for (px, py) in [(x + d, y), (x, y + d)] {
            if px >= 0 && py >= 0 && px < w as i32 && py < h as i32 {
                rgb.put_pixel(px as u32, py as u32, image::Rgb([18, 255, 180]));
            }
        }
    }
    if w > 0 && h > 0 {
        rgb.put_pixel(x as u32, y as u32, image::Rgb([255, 255, 255]));
    }
    encode(rgb.as_raw(), w, h)
}
