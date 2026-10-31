//! D435 color-node detection adapted from Kinova-VLA/crates/webcams.
use nokhwa::utils::FrameFormat;
use std::os::fd::AsRawFd;

pub struct Device {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub format: FrameFormat,
}

#[repr(C)]
struct Format {
    index: u32,
    type_: u32,
    flags: u32,
    description: [u8; 32],
    pixelformat: u32,
    mbus_code: u32,
    reserved: [u32; 3],
}

#[repr(C)]
struct FrameSize {
    index: u32,
    pixel_format: u32,
    type_: u32,
    width: u32,
    height: u32,
    step_width: u32,
    step_height: u32,
    union_tail: [u32; 2],
    reserved: [u32; 2],
}

fn resolution(sizes: &[(u32, u32)]) -> (u32, u32) {
    if sizes.contains(&(640, 480)) {
        (640, 480)
    } else {
        sizes.first().copied().unwrap_or((640, 480))
    }
}

pub fn detect() -> Option<Device> {
    let mut nodes: Vec<u32> = std::fs::read_dir("/sys/class/video4linux")
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()?
                .strip_prefix("video")?
                .parse()
                .ok()
        })
        .collect();
    nodes.sort_unstable();
    for index in nodes {
        let Ok(name) = std::fs::read_to_string(format!("/sys/class/video4linux/video{index}/name"))
        else {
            continue;
        };
        if !name.contains("RealSense") {
            continue;
        }
        let Ok(file) = std::fs::File::open(format!("/dev/video{index}")) else {
            continue;
        };
        let fd = file.as_raw_fd();
        let yuyv = u32::from_le_bytes(*b"YUYV");
        let mut supported = false;
        for i in 0..64 {
            let mut desc = Format {
                index: i,
                type_: 1,
                flags: 0,
                description: [0; 32],
                pixelformat: 0,
                mbus_code: 0,
                reserved: [0; 3],
            };
            // SAFETY: fd is open and desc has the kernel's v4l2_fmtdesc layout.
            if unsafe { libc::ioctl(fd, 0xC0405602 as libc::c_ulong, &mut desc) } < 0 {
                break;
            }
            if desc.pixelformat == yuyv {
                supported = true;
                break;
            }
        }
        if !supported {
            continue;
        }
        let mut sizes = Vec::new();
        for i in 0..64 {
            let mut size = FrameSize {
                index: i,
                pixel_format: yuyv,
                type_: 0,
                width: 0,
                height: 0,
                step_width: 0,
                step_height: 0,
                union_tail: [0; 2],
                reserved: [0; 2],
            };
            // SAFETY: fd is open and size has the kernel's v4l2_frmsizeenum layout.
            if unsafe { libc::ioctl(fd, 0xC02C564A as libc::c_ulong, &mut size) } < 0 {
                break;
            }
            if size.type_ == 1 {
                sizes.push((size.width, size.height));
            }
        }
        let (width, height) = resolution(&sizes);
        return Some(Device {
            index,
            width,
            height,
            fps: 30,
            format: FrameFormat::YUYV,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefers_vlm_resolution_and_matches_kernel_layouts() {
        assert_eq!(resolution(&[(1280, 720), (640, 480)]), (640, 480));
        assert_eq!(resolution(&[(1280, 720)]), (1280, 720));
        assert_eq!(resolution(&[]), (640, 480));
        assert_eq!(std::mem::size_of::<Format>(), 64);
        assert_eq!(std::mem::size_of::<FrameSize>(), 44);
    }
}
