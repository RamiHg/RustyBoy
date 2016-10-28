
// We allow dead code for now - eventually I'll remove this as the CPU is hooked up
#![allow(dead_code)]

mod cpu;
mod memory;
mod alu;
mod cart;
mod gpu;
mod system;
mod debug;

#[macro_use]
extern crate glium;
extern crate image;

use glium::{DisplayBuild, Surface};
use glium::glutin;

use system::System;
use gpu::*;
use memory::*;

use std::borrow::Cow;

fn main() {
    let mut system = System::new();
    system.start_system("/Users/ramy/Desktop/opus5.gb");
    //system.start_system("/Users/ramy/Desktop/cpu_instrs.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/04-op r,imm.gb");

    let display = glutin::WindowBuilder::new()
        .build_glium()
        .unwrap();
        
    let back_buffer = glium::Texture2d::empty_with_format(
        &display,
        glium::texture::UncompressedFloatFormat::U8U8U8U8,
        glium::texture::MipmapsOption::NoMipmap,
        160, 144).unwrap();
    back_buffer.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);

    loop 
    {
        for i in 0..200 {
            system.execute_instruction();
        }

        let target = display.draw();

        //if system.gpu.mode == GpuMode::VBlank {
        {
            let mut data: [u8; 160*144 * 3] = [0; 160*144 * 3];

            for j in 0..144_usize {
                for i in 0..160_usize {
                    let pixel = system.gpu.get_pixel(i as u32, j as u32);

                    data[(i + j*160) * 3 + 0] = pixel.r;
                    data[(i + j*160) * 3 + 1] = pixel.g;
                    data[(i + j*160) * 3 + 2] = pixel.b;
                }
            }

            let raw_image = glium::texture::RawImage2d {
                data: Cow::Borrowed(&data),
                width: 160,
                height: 144,
                format: glium::texture::ClientFormat::U8U8U8,
            };
            
            let image = glium::Texture2d::with_format(
                &display,
                raw_image,
                glium::texture::UncompressedFloatFormat::U8U8U8U8,
                glium::texture::MipmapsOption::NoMipmap
            ).unwrap();

            image.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Nearest);
        }

        //back_buffer.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
        target.finish().unwrap();
        
        for event in display.poll_events() {
            match event {
                glutin::Event::Closed => break,
                _ => ()
            }
        }
    }
}
