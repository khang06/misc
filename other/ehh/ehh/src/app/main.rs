use gl::types::*;
use log::{error, info};
use std::{
    cell::RefCell,
    ffi::{c_void, CStr},
    rc::Rc,
    time::Instant,
};

use crate::framework::{
    bass::Bass,
    render::{Alignment, DrawBatch, TextRenderer, TextSprite},
};

use super::game::OsuGame;

extern "system" fn gl_msg_callback(
    source: GLenum,
    gltype: GLenum,
    id: GLuint,
    severity: GLenum,
    _length: GLsizei,
    message: *const GLchar,
    _user_param: *mut c_void,
) {
    unsafe {
        let source_str = match source {
            gl::DEBUG_SOURCE_API => "API".to_string(),
            x => format!("Unknown {x}"),
        };
        let type_str = match gltype {
            gl::DEBUG_TYPE_ERROR => "ERROR".to_string(),
            gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "DEPRECATED BEHAVIOR".to_string(),
            gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "UNDEFINED BEHAVIOR".to_string(),
            gl::DEBUG_TYPE_PORTABILITY => "PORTABILITY".to_string(),
            gl::DEBUG_TYPE_PERFORMANCE => "PERFORMANCE".to_string(),
            gl::DEBUG_TYPE_MARKER => "MARKER".to_string(),
            gl::DEBUG_TYPE_PUSH_GROUP => "PUSH GROUP".to_string(),
            gl::DEBUG_TYPE_POP_GROUP => "POP GROUP".to_string(),
            gl::DEBUG_TYPE_OTHER => "OTHER".to_string(),
            x => format!("Unknown {x}"),
        };

        let out = format!(
            "{id}: {type_str} from {source_str}: {}",
            CStr::from_ptr(message).to_string_lossy()
        );
        match severity {
            gl::DEBUG_SEVERITY_HIGH => log::error!("{}", out),
            gl::DEBUG_SEVERITY_MEDIUM => log::warn!("{}", out),
            gl::DEBUG_SEVERITY_LOW => log::info!("{}", out),
            gl::DEBUG_SEVERITY_NOTIFICATION => log::info!("{}", out),
            _ => unreachable!(),
        }
    }
}

struct FPSCounter {
    renderer: Rc<RefCell<TextRenderer>>,
    frame_start: Instant,
    text: TextSprite,
}

impl FPSCounter {
    pub fn new(renderer: Rc<RefCell<TextRenderer>>, width: f32, height: f32) -> FPSCounter {
        FPSCounter {
            renderer: renderer.clone(),
            frame_start: Instant::now(),
            text: TextSprite::new(
                renderer,
                "0 fps\n0.0 ms",
                width - 4.0,
                height - 52.0,
                0.25,
                Alignment::Right,
            ),
        }
    }

    pub fn end_frame(&mut self) {
        let ms = self.frame_start.elapsed().as_secs_f32();
        let fps = (1.0 / ms) as i64;
        self.frame_start = Instant::now();
        self.text
            .set_text(&format!("{fps} fps\n{:.1} ms", ms * 1000.0));
    }

    pub fn draw(&mut self, batch: &mut DrawBatch) {
        self.text.add_to_batch(batch);
    }
}

pub struct EhhApp {
    bass: Rc<Bass>,
    text_renderer: Rc<RefCell<TextRenderer>>,
    batch: DrawBatch,
    fps_counter: FPSCounter,
    game: OsuGame,
}

impl EhhApp {
    pub fn run(beatmap_path: String) {
        let width = 1920;
        let height = 1080;

        let sdl = sdl2::init().expect("Failed to initialize SDL2");
        let sdl_video = sdl
            .video()
            .expect("Failed to initialized SDL2's video subsystem");
        let gl_attr = sdl_video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(4, 6);
        let mut window = sdl_video
            .window("ehh", width, height)
            .opengl()
            .fullscreen()
            .position_centered()
            .build()
            .expect("Failed to create a window");
        let _gl_context = window
            .gl_create_context()
            .expect("Failed to initialize a GL context");
        gl::load_with(|s| sdl_video.gl_get_proc_address(s) as *const c_void);

        sdl_video.gl_set_swap_interval(0).unwrap();

        #[cfg(debug_assertions)]
        unsafe {
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(gl_msg_callback), std::ptr::null());
        }

        let bass = Rc::new(Bass::new(-1, 44100, 0).expect("Failed to initialize BASS"));
        let bass_version = bass.get_version();
        let bassmix_version = bass.get_bassmix_version();
        info!(
            "BASS version:    {}.{}.{}.{}",
            bass_version.0, bass_version.1, bass_version.2, bass_version.3
        );
        info!(
            "BASSmix version: {}.{}.{}.{}",
            bassmix_version.0, bassmix_version.1, bassmix_version.2, bassmix_version.3
        );
        info!(
            "Using device:    {}",
            bass.get_device_info(bass.get_device().unwrap())
                .unwrap()
                .name
        );

        // osu beatmap sync stuff
        bass.set_config(68, 1).unwrap(); // BASS_CONFIG_MP3_OLDGAPS (undocumented in bass.chm!!!)
        bass.set_config(bass_sys::BASS_CONFIG_DEV_NONSTOP, 1)
            .unwrap();
        bass.set_config(bass_sys::BASS_CONFIG_UPDATEPERIOD, 5);
        bass.set_config(bass_sys::BASS_CONFIG_UPDATETHREADS, 1);

        let text_renderer = Rc::new(RefCell::new(TextRenderer::new(&[
            include_bytes!("../../assets/fonts/NotoSans-Bold.ttf").to_vec(),
            include_bytes!("../../assets/fonts/NotoSansJP-Bold.otf").to_vec(),
        ])));

        let game = match OsuGame::new(
            bass.clone(),
            text_renderer.clone(),
            width as f32,
            height as f32,
            beatmap_path,
        ) {
            Ok(x) => x,
            Err(x) => {
                error!("Failed to create the game scene: {x}");
                return;
            }
        };

        let ortho = cgmath::ortho(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);

        let mut app = EhhApp {
            bass,
            fps_counter: FPSCounter::new(text_renderer.clone(), width as f32, height as f32),
            batch: DrawBatch::new(ortho),
            text_renderer,
            game,
        };
        window.set_title(&app.game.get_title()).unwrap();

        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);

            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let mut event_pump = sdl.event_pump().unwrap();
        'main: loop {
            for event in event_pump.poll_iter() {
                match event {
                    sdl2::event::Event::Quit { .. } => break 'main,
                    _ => {}
                }
            }

            unsafe {
                gl::Clear(gl::COLOR_BUFFER_BIT);
            }

            app.game.update();
            app.game.draw();

            app.fps_counter.draw(&mut app.batch);
            app.batch.draw();

            app.fps_counter.end_frame();

            window.gl_swap_window();
        }
    }
}
