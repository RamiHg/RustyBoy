use gl::types::GLuint;

use crate::gpu;

type EventLoop = glutin::event_loop::EventLoop<()>;

#[macro_export]
macro_rules! GL {
    ($x:stmt) => {
        $x
        let error = gl::GetError();
        assert!(error == 0, "GL error in: {:?}", error);
    };
}

const FULLSCREEN_VERT_SHADER: &str = "
#version 150 core
out vec2 uv;
void main() {
    gl_Position.xy = -1 + vec2(
        (gl_VertexID & 1) << 2,
        (gl_VertexID & 2) << 1);
    gl_Position.zw = vec2(-1, 1);
    uv = (gl_Position.xy + 1) / 2;
    // Flip image vertically because we are writing it with top-left origin.
    //uv.y = 1 - uv.y;
}
\0
";

const FRAG_BLIT_SHADER: &str = "
#version 150 core
in vec2 uv;
out vec3 color;
uniform sampler2D gpu_tex;
void main() {
    // Flip image vertically because we are writing it with top-left origin.
    vec2 flipped = vec2(uv.x, 1 - uv.y);
    color = texture(gpu_tex, flipped).rgb;
}
\0
";

fn compile_shader(shader: GLuint, source: &str) {
    unsafe {
        GL!(gl::ShaderSource(shader, 1, [source.as_ptr() as *const _].as_ptr(), core::ptr::null()));
        GL!(gl::CompileShader(shader));
        let mut info_length = 0;
        GL!(gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut info_length));
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
        GL!(gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut info_length));
        if info_length > 0 {
            let mut log = String::from_utf8(vec![0; info_length as usize]).unwrap();
            GL!(gl::GetProgramInfoLog(
                program_id,
                info_length,
                core::ptr::null_mut(),
                log.as_mut_str() as *mut _ as *mut _
            ));
            println!("{}", log);
        }
        let mut link_status = 0;
        GL!(gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut link_status));
        assert_eq!(link_status, i32::from(gl::TRUE), "Linking failed. See log above.");
        GL!(gl::DetachShader(program_id, vert_shader));
        GL!(gl::DetachShader(program_id, frag_shader));
        GL!(gl::DeleteShader(vert_shader));
        GL!(gl::DeleteShader(frag_shader));
        program_id
    }
}

pub struct Window {
    context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    shader: GLuint,
}

impl Window {
    pub fn with_event_loop(event_loop: &EventLoop) -> Window {
        let window = glutin::window::WindowBuilder::new().with_title("RustyBoy");
        let context = glutin::ContextBuilder::new()
            .with_vsync(false)
            .with_gl(glutin::GlRequest::Latest)
            .with_gl_profile(glutin::GlProfile::Core)
            .build_windowed(window, event_loop)
            .unwrap();

        let context = unsafe { context.make_current().unwrap() };
        gl::load_with(|s| context.get_proc_address(s) as *const _);

        // Create a dummy VAO.
        let mut dummy_vao = 0_u32;
        unsafe {
            GL!(gl::GenVertexArrays(1, &mut dummy_vao));
            GL!(gl::BindVertexArray(dummy_vao));
        }

        // Load the shaders.
        let fs_copy = load_all_shaders();

        let filter = gl::NEAREST;
        // Create our GPU target image.
        let mut target_image: GLuint = 0;
        unsafe {
            GL!(gl::GenTextures(1, &mut target_image));
            assert!(target_image != gl::INVALID_VALUE);
            GL!(gl::ActiveTexture(gl::TEXTURE0));
            GL!(gl::BindTexture(gl::TEXTURE_2D, target_image));
            GL!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                gpu::LCD_WIDTH as i32,
                gpu::LCD_HEIGHT as i32,
                0,
                gl::BGRA,
                gl::UNSIGNED_INT_8_8_8_8_REV,
                core::ptr::null(),
            ));
            GL!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32));
            GL!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter as i32));
        }
        Window { context, shader: fs_copy }
    }

    pub fn update_screen(&self, pixels: &[u8]) {
        assert_eq!(pixels.len(), gpu::LCD_WIDTH * gpu::LCD_HEIGHT * 4);
        unsafe {
            GL!(gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                gpu::LCD_WIDTH as i32,
                gpu::LCD_HEIGHT as i32,
                gl::BGRA,
                gl::UNSIGNED_INT_8_8_8_8_REV,
                pixels.as_ptr() as *const core::ffi::c_void
            ));

            // Copy GPU image to framebuffer.
            GL!(gl::UseProgram(self.shader));
            GL!(gl::DrawArrays(gl::TRIANGLES, 0, 3));
            GL!(gl::DisableVertexAttribArray(0));
        }
    }

    pub fn request_redraw(&self) {
        self.context.window().request_redraw();
    }

    pub fn swap_buffers(&self) {
        self.context.swap_buffers().unwrap();
    }
}
