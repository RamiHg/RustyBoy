use std::path::Path;

use super::base_path_to;
use crate::gpu::{self, LCD_HEIGHT, LCD_WIDTH};
use crate::system::System;

impl Into<bmp::Pixel> for gpu::Pixel {
    fn into(self) -> bmp::Pixel {
        use bmp::{px, Pixel};
        px!(self.r, self.g, self.b)
    }
}

pub fn dump_system_image(sub_dir: &Path, test_name: &str, system: &System) {
    dump_image(sub_dir, test_name, system.screen());
}

pub fn dump_image(sub_dir: &Path, test_name: &str, screen: &[gpu::Color]) {
    use bmp::Image;
    let test_name_path = Path::new(test_name);
    let mut path = base_path_to(sub_dir);
    std::fs::create_dir_all(path.join(test_name_path.parent().unwrap_or_else(|| Path::new("."))))
        .unwrap();
    path.push(format!("{}.bmp", test_name));
    let mut img = Image::new(LCD_WIDTH as u32, LCD_HEIGHT as u32);
    for j in 0..LCD_HEIGHT as usize {
        for i in 0..LCD_WIDTH as usize {
            let pixel: gpu::Pixel = gpu::Pixel::from(&screen[i + j * LCD_WIDTH as usize]);
            img.set_pixel(i as u32, j as u32, pixel.into());
        }
    }
    img.save(path).unwrap();
}
