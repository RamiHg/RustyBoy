use super::base_path_to;
use crate::gpu::{self, LCD_HEIGHT, LCD_WIDTH};
use crate::system::System;


use std::path::{Path, PathBuf};

impl From<bmp::Pixel> for gpu::Pixel {
    fn from(pixel: bmp::Pixel) -> gpu::Pixel {
        gpu::Pixel {
            r: pixel.r,
            g: pixel.g,
            b: pixel.b,
        }
    }
}

impl Into<bmp::Pixel> for gpu::Pixel {
    fn into(self) -> bmp::Pixel {
        use bmp::{px, Pixel};
        px!(self.r, self.g, self.b)
    }
}

pub fn load_golden_image(test_name: &str) -> Vec<gpu::Pixel> {
    use bmp::{Image, Pixel};
    use std::path::PathBuf;

    let mut path = base_path_to("./test_golden/mooneye");
    path.push(format!("{}.bmp", test_name));

    let img = bmp::open(path).unwrap();
    assert_eq!(img.get_width(), LCD_WIDTH as u32);
    assert_eq!(img.get_height(), LCD_HEIGHT as u32);
    let mut result = Vec::with_capacity((LCD_WIDTH * LCD_HEIGHT) as usize);
    for j in 0..LCD_HEIGHT {
        for i in 0..LCD_WIDTH {
            let src_pixel = img.get_pixel(i as u32, j as u32);
            result.push(gpu::Pixel::from(src_pixel));
        }
    }
    result
}

pub fn dump_system_image(sub_dir: &Path, test_name: &str, system: &System) {
    dump_image(sub_dir, test_name, system.get_screen());
}

pub fn dump_image(sub_dir: &Path, test_name: &str, screen: &[gpu::Pixel]) {
    use bmp::{Image, Pixel};
    let test_name_path = Path::new(test_name);
    let mut path = base_path_to(sub_dir);
    std::fs::create_dir_all(path.join(test_name_path.parent().unwrap_or(Path::new(".")))).unwrap();
    path.push(format!("{}.bmp", test_name));
    let mut img = Image::new(LCD_WIDTH as u32, LCD_HEIGHT as u32);
    for j in 0..LCD_HEIGHT as usize {
        for i in 0..LCD_WIDTH as usize {
            img.set_pixel(
                i as u32,
                j as u32,
                screen[i + j * LCD_WIDTH as usize].into(),
            );
        }
    }
    img.save(path).unwrap();
}
