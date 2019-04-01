// We allow dead code for now - eventually I'll remove this as the CPU is hooked up
#![allow(dead_code)]
#![deny(warnings)]
#![deny(clippy::all)]
// Annoying, and wrong, clippy warning regarding FromPrimitive.
#![allow(clippy::useless_attribute)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(unused_doc_comments)]
// Remove when this file is uncommented out.
#![allow(unused_variables)]
// Temp.
#![allow(unused_imports)]

use gl::types::GLuint;

mod cart;
mod cpu;
mod gpu;
mod io_registers;
mod memory;
mod system;
mod util;

#[allow(unused_imports)]
use system::System;

// Helpful links:
// Cycle-accurate docs: https://github.com/AntonioND/giibiiadvance/blob/master/docs/TCAGBD.pdf
// https://github.com/gbdev/awesome-gbdev#emulator-development
// https://www.youtube.com/watch?v=HyzD8pNlpwI
// https://www.youtube.com/watch?v=GBYwjch6oEE

/// Helpful macro to run a GL command and make sure no errors are generated.
macro_rules! GL {
    ($x:stmt) => {
        $x;
        let error = gl::GetError();
        assert!(error == 0, "GL error in: {:?}", error);
    };
}

const FULLSCREEN_VERT_SHADER: &str = "
#version 410 core
out vec2 uv;
void main() {
    gl_Position.xy = -1 + vec2(
        (gl_VertexID & 1) << 2,
        (gl_VertexID & 2) << 1);
    gl_Position.zw = vec2(-1, 1);
    uv = (gl_Position.xy + 1) / 2;
}
\0
";

const FRAG_BLIT_SHADER: &str = "
#version 410 core
in vec2 uv;
out vec3 color;
uniform sampler2D gpu_tex;
void main() {
    color = texture(gpu_tex, uv).rgb;
}
\0
";

fn compile_shader(shader: GLuint, source: &str) {
    unsafe {
        GL!(gl::ShaderSource(
            shader,
            1,
            [source.as_ptr() as *const _].as_ptr(),
            core::ptr::null()
        ));
        GL!(gl::CompileShader(shader));
        let mut info_length = 0;
        GL!(gl::GetShaderiv(
            shader,
            gl::INFO_LOG_LENGTH,
            &mut info_length
        ));
        if info_length > 0 {
            let mut log = String::from_utf8(vec![0; info_length as usize]).unwrap();
            GL!(gl::GetShaderInfoLog(
                shader,
                info_length,
                core::ptr::null_mut(),
                log.as_mut_str() as *mut _ as *mut _
            ));
            panic!("Could not compile shader: \n{}", log);
        }
    }
}

fn load_all_shaders() -> GLuint {
    unsafe {
        let vert_shader = gl::CreateShader(gl::VERTEX_SHADER);
        assert!(vert_shader != 0);
        compile_shader(vert_shader, FULLSCREEN_VERT_SHADER);

        let frag_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        assert!(frag_shader != 0);
        compile_shader(frag_shader, FRAG_BLIT_SHADER);

        let program_id = gl::CreateProgram();
        assert!(program_id != 0);
        GL!(gl::AttachShader(program_id, vert_shader));
        GL!(gl::AttachShader(program_id, frag_shader));
        GL!(gl::LinkProgram(program_id));
        let mut info_length = 0;
        GL!(gl::GetProgramiv(
            program_id,
            gl::INFO_LOG_LENGTH,
            &mut info_length
        ));
        if info_length > 0 {
            let mut log = String::from_utf8(vec![0; info_length as usize]).unwrap();
            GL!(gl::GetProgramInfoLog(
                program_id,
                info_length,
                core::ptr::null_mut(),
                log.as_mut_str() as *mut _ as *mut _
            ));
            panic!("Could not link program: \n{}", log);
        }
        GL!(gl::DetachShader(program_id, vert_shader));
        GL!(gl::DetachShader(program_id, frag_shader));
        GL!(gl::DeleteShader(vert_shader));
        GL!(gl::DeleteShader(frag_shader));
        program_id
    }
}

fn main() {
    use glutin::GlContext;

    // http://gbdev.gg8.se/wiki/articles/Test_ROMs
    //let cart = "/Users/ramy/Desktop/opus5.gb";
    //let cart = "/Users/ramy/Desktop/testroms/cpu_instrs/cpu_instrs.gb";
    let cart = "/Users/ramy/Desktop/testroms/cpu_instrs/individual/03-op sp,hl.gb";
    //let cart = "/Users/ramy/Desktop/testroms/cpu_instrs/individual/04-op r,imm.gb";

    //let mut system = System::new(cart);
    //system.start_system();
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/02-interrupts.gb");
    //system.start_system("");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/04-op r,imm.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/05-op rp.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/06-ld r,r.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/08-misc instrs.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/09-op r,r.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/10-bit ops.gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/11-op a,(hl).gb");
    //system.start_system("/Users/ramy/Downloads/cpu_instrs/individual/01-special.gb");

    //system.start_system();

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (4, 1)))
        .with_gl_profile(glutin::GlProfile::Core);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
        assert!(!gl_window.is_current());
    }

    gl::load_with(|s| gl_window.get_proc_address(s) as *const _);

    // Create a dummy VAO.
    let mut dummy_vao = 0_u32;
    unsafe {
        GL!(gl::GenVertexArrays(1, &mut dummy_vao));
        GL!(gl::BindVertexArray(dummy_vao));
    }
    // Load the shaders.
    let fs_copy = load_all_shaders();

    // Create our GPU target image.
    let mut target_image: GLuint = 0;
    unsafe {
        GL!(gl::GenTextures(1, &mut target_image));
        assert!(target_image != gl::INVALID_VALUE);
        GL!(gl::BindTexture(gl::TEXTURE_2D, target_image));
        GL!(gl::TexStorage2D(gl::TEXTURE_2D, 1, gl::RGB8, 160, 144));
        GL!(gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::NEAREST as i32
        ));
        GL!(gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32
        ));
    }

    loop {
        for _ in 0..2000 {
            // system.execute_instruction();
        }

        //if system.gpu.mode == GpuMode::VBlank {
        {
            let /*mut*/ data: [u8; 160 * 144 * 3] = [0; 160 * 144 * 3];

            for j in 0..144_usize {
                for i in 0..160_usize {
                    // let pixel = system.gpu.get_pixel(i as u32, j as u32);

                    // data[(i + j * 160) * 3] = pixel.r;
                    // data[(i + j * 160) * 3 + 1] = pixel.g;
                    // data[(i + j * 160) * 3 + 2] = pixel.b;
                }
            }

            unsafe {
                GL!(gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    160,
                    144,
                    gl::RGB,
                    gl::UNSIGNED_BYTE,
                    data.as_ptr() as *const core::ffi::c_void
                ));
            }
        }

        // Copy GPU image to framebuffer.
        unsafe {
            GL!(gl::UseProgram(fs_copy));
            GL!(gl::DrawArrays(gl::TRIANGLES, 0, 3));
            GL!(gl::DisableVertexAttribArray(0));
        }

        let mut running = true;
        #[allow(clippy::single_match)]
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

        gl_window.swap_buffers().unwrap();
    }
}
