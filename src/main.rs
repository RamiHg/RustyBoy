
// We allow dead code for now - eventually I'll remove this as the CPU is hooked up
#![allow(dead_code)]

mod cpu;
mod memory;
mod alu;
mod cart;
mod gpu;

#[macro_use]
extern crate glium;
extern crate image;

use glium::{DisplayBuild, Surface};
use glium::glutin;

fn main() {
    let display = glutin::WindowBuilder::new()
        .build_glium()
        .unwrap();
        
    let back_buffer = glium::Texture2d::empty_with_format(
        &display,
        glium::texture::UncompressedFloatFormat::U8U8U8U8,
        glium::texture::MipmapsOption::NoMipmap,
        160, 144).unwrap();
    back_buffer.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);
    
    for event in display.wait_events() {
        let target = display.draw();
        back_buffer.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
        target.finish().unwrap();
      
        
        match event {
            glutin::Event::Closed => break,
            _ => ()
        }
    }
}
