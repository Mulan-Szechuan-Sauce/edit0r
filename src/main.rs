mod mode;
mod text_mode;

use mode::MajorMode;
use text_mode::TextMode;

use std::path::Path;

use sdl2::Sdl;
use sdl2::render::Canvas;
use sdl2::ttf::Font;
use sdl2::video::Window;

pub struct RenderContext<'a> {
    sdl: &'a Sdl,
    canvas: &'a mut Canvas<Window>,
    font: &'a Font<'a, 'a>,
}

fn main() -> Result<(), String> {
    println!("test"); // Wonderful comment
    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    static SCREEN_WIDTH: u32 = 1000;
    static SCREEN_HEIGHT: u32 = 800;
    let window = video_subsys
        .window("edit0r", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    // Load a font
    let font_path = Path::new("assets/VeraMono.ttf");
    let font = ttf_context.load_font(font_path, 20)?;

    let mut context = RenderContext {
        sdl: &sdl_context,
        canvas: &mut canvas,
        font: &font,
    };

    TextMode {}.draw(&mut context)
}
