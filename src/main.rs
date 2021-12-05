use std::env;
use std::path::Path;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::render::TextureQuery;
use sdl2::ttf::Font;
use sdl2::video::Window;

#[allow(dead_code)]
#[derive(Clone)]
enum FontColor {
    Default,
    Rgb(u8, u8, u8),
}

#[allow(dead_code)]
#[derive(Clone)]
struct FontFace {
    fg_color: FontColor,
    bg_color: FontColor,
}

#[allow(dead_code)]
struct TextContent {
    lines: Vec<Vec<(FontFace, String)>>,
}

#[allow(dead_code)]
struct RenderContext<'a> {
    canvas: &'a mut Canvas<Window>,
    font: &'a Font<'a, 'a>,
}

static SCREEN_WIDTH: u32 = 800;
static SCREEN_HEIGHT: u32 = 600;

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

// Returns the height of the rendered line
fn draw_line(
    context: &mut RenderContext,
    y_offset: u32,
    line: &Vec<(FontFace, String)>
) -> Result<u32, String> {
    let mut line_height = 0;
    let mut x_offset = 0;

    let empty_line = " ".to_string();

    for (face, _segment) in line {
        let mut segment = _segment;

        if _segment.len() == 0 {
            segment = &empty_line;
        }

        let fg_color = match &face.fg_color {
            FontColor::Default      => Color::RGB(255, 255, 255),
            FontColor::Rgb(r, g, b) => Color::RGB(*r, *g, *b),
        };

        let texture_creator = context.canvas.texture_creator();

        // render a surface, and convert it to a texture bound to the canvas
        let surface = context.font
            .render(&segment)
            .blended(fg_color)
            .map_err(|e| e.to_string())?;
        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .map_err(|e| e.to_string())?;

        let TextureQuery { width, height, .. } = texture.query();
        let target = rect!(x_offset, y_offset, width, height);

        match &face.bg_color {
            FontColor::Rgb(r, g, b) => {
                context.canvas.set_draw_color(Color::RGB(*r, *g, *b));
                context.canvas.fill_rect(target)?;
            },
            _ => {},
        };

        context.canvas.copy(&texture, None, Some(target))?;

        x_offset += width;
        line_height = height;
    }

    Ok(line_height)
}

fn draw_content(context: &mut RenderContext, content: &TextContent) -> Result<(), String> {
    let mut y_offset = 0;

    for line in &content.lines {
        y_offset += draw_line(context, y_offset, &line)?;
    }

    Ok(())
}

fn add_line_numbers(content: TextContent) -> TextContent {
    let mut new_lines = vec!();
    let max_digits = (content.lines.len() + 1) % 10;

    for i in 0..content.lines.len() {
        let mut segments = vec!();
        segments.push((
            FontFace {
                bg_color: FontColor::Default,
                fg_color: FontColor::Rgb(150, 150, 150),
            },
            format!("{:width$} ", i + 1, width = max_digits)
        ));
        segments.append(&mut content.lines[i].clone());
        new_lines.push(segments);
    }

    TextContent {
        lines: new_lines,
    }
}

fn run() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

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

    let content = std::fs::read_to_string("src/main.rs")
        .map_err(|e| e.to_string())?;

    let mut lines: Vec<Vec<(FontFace, String)>> = vec!();

    for line in content.lines() {
        let mut segments: Vec<(FontFace, String)> = vec!();

        if line.starts_with("use ") {
            segments.push((
                FontFace {
                    fg_color: FontColor::Rgb(255, 150, 150),
                    bg_color: FontColor::Rgb(0, 100, 0),
                },
                "use ".to_string()
            ));
            segments.push((
                FontFace {
                    fg_color: FontColor::Default,
                    bg_color: FontColor::Default,
                },
                line[4..].to_string()
            ));
        } else {
            segments.push((
                FontFace {
                    fg_color: FontColor::Default,
                    bg_color: FontColor::Default,
                },
                line.to_string()
            ));
        }
        
        lines.push(segments);
    }

    let text_content = add_line_numbers(TextContent {
        lines: lines,
    });

    'mainloop: loop {
        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGBA(0, 0, 0, 0));
        canvas.clear();

        let mut context = RenderContext {
            canvas: &mut canvas,
            font: &font,
        };

        draw_content(&mut context, &text_content)?;

        canvas.present();
    }

    Ok(())
}

fn main() -> Result<(), String> {
    run()
}
