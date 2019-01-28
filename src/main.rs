// We allow dead code for now - eventually I'll remove this as the CPU is hooked up
#![allow(dead_code)]
#![deny(warnings)]

use gl;
use glutin;
//use std::borrow::Cow;

mod alu;
mod cart;
mod cpu;
mod debug;
mod gpu;
mod memory;
mod registers;
mod system;

use system::System;

fn main() {
    //use gl::types::*;
    use glutin::GlContext;

    let mut system = System::new();
    system.start_system("/Users/ramy/Desktop/opus5.gb");
    //system.start_system("/Users/ramy/Desktop/cpu_instrs.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/02-interrupts.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/03-op sp,hl.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/04-op r,imm.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/05-op rp.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/06-ld r,r.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/08-misc instrs.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/09-op r,r.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/10-bit ops.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/11-op a,(hl).gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/01-special.gb");

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)));
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
        gl::load_with(|s| gl_window.get_proc_address(s) as *const _);
    }
    gl::load_with(|s| gl_window.get_proc_address(s) as *const _);

    // Create our GPU target image.
    //gl::

    // let back_buffer = glium::Texture2d::empty_with_format(
    //     &display,
    //     glium::texture::UncompressedFloatFormat::U8U8U8U8,
    //     glium::texture::MipmapsOption::NoMipmap,
    //     160,
    //     144,
    // )
    // .unwrap();
    // back_buffer.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);

    loop {
        for _ in 0..2000 {
            system.execute_instruction();
        }

        //let target = display.draw();

        //if system.gpu.mode == GpuMode::VBlank {
        {
            let mut data: [u8; 160 * 144 * 3] = [0; 160 * 144 * 3];

            for j in 0..144_usize {
                for i in 0..160_usize {
                    let pixel = system.gpu.get_pixel(i as u32, j as u32);

                    data[(i + j * 160) * 3 + 0] = pixel.r;
                    data[(i + j * 160) * 3 + 1] = pixel.g;
                    data[(i + j * 160) * 3 + 2] = pixel.b;
                }
            }

            // let raw_image = glium::texture::RawImage2d {
            //     data: Cow::Borrowed(&data),
            //     width: 160,
            //     height: 144,
            //     format: glium::texture::ClientFormat::U8U8U8,
            // };

            // let image = glium::Texture2d::with_format(
            //     &display,
            //     raw_image,
            //     glium::texture::UncompressedFloatFormat::U8U8U8U8,
            //     glium::texture::MipmapsOption::NoMipmap,
            // )
            // .unwrap();

            // image
            //     .as_surface()
            //     .fill(&target, glium::uniforms::MagnifySamplerFilter::Nearest);
        }

        //back_buffer.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
        //target.finish().unwrap();

        let mut running = true;
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => running = false,
                _ => (),
            },
            _ => (),
        });
        if !running {
            break;
        }
    }
}
