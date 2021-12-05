use tree_sitter::Node;
use tree_sitter::QueryCursor;
use tree_sitter::Query;
use tree_sitter::Point;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use tree_sitter::Parser;

use crate::RenderContext;
use crate::mode::MajorMode;

pub trait TextMinorMode {
    fn modify(&mut self, lines: &mut TextContent);
}

#[derive(Clone, PartialEq)]
pub enum FontColor {
    Default,
    Rgb(u8, u8, u8),
    // TODO:? ThemeColor(usize),
}

#[derive(Clone, PartialEq)]
pub struct FontFace {
    fg_color: FontColor,
    bg_color: FontColor,
}

impl Default for FontFace {
    fn default() -> FontFace {
        FontFace {
            fg_color: FontColor::Default,
            bg_color: FontColor::Default,
        }
    }
}

pub struct LineSegment {
    row: usize,
    col_begin: usize,
    col_end: usize,
}

// TODO: Add margins
pub struct TextContent {
    // Dumb character by character face mapping
    faces: Vec<Vec<FontFace>>,
    lines: Vec<String>,
}

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

fn draw_segment(
    context: &mut RenderContext,
    x_offset: u32,
    y_offset: u32,
    face: &FontFace,
    text: &str
) -> Result<u32, String> {
    let fg_color = match &face.fg_color {
        FontColor::Default      => Color::RGB(255, 255, 255),
        FontColor::Rgb(r, g, b) => Color::RGB(*r, *g, *b),
    };

    let texture_creator = context.canvas.texture_creator();

    // render a surface, and convert it to a texture bound to the canvas
    let surface = context.font
        .render(text)
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
    Ok(x_offset + width)
}

// Returns the height of the rendered line
fn draw_line(
    context: &mut RenderContext,
    y_offset: u32,
    faces: &Vec<FontFace>,
    line: &String,
) -> Result<u32, String> {
    let mut x_offset = 0;
    let mut current_face: &FontFace = &Default::default();

    if line.len() != faces.len() {
        panic!("Line length must equal face length");
    }

    let mut segment_start = 0;
    let mut segment_len = 0;

    for col in 0..line.len() {
        let char_face = &faces[col];

        if char_face == current_face {
            segment_len += 1;
        } else {
            if segment_len > 0 {
                x_offset += draw_segment(
                    context, x_offset, y_offset, current_face,
                    &line[segment_start..segment_start + segment_len + 1])?;
            }

            segment_len = 0;
            segment_start = col;
            current_face = char_face;
        }
    }

    if segment_len > 0 {
        draw_segment(
            context, x_offset, y_offset, current_face,
            &line[segment_start..])?;
    }

    Ok(context.font.size_of_char('a').unwrap().1)
}

fn draw_content(context: &mut RenderContext, content: &TextContent) -> Result<(), String> {
    let mut y_offset = 0;
    for i in 0..content.lines.len() {
        let line = &content.lines[i];
        y_offset += match content.faces.get(i) {
            Some(faces) => draw_line(context, y_offset, faces, line)?,
            None        => draw_line(context, y_offset, &vec!(), line)?,
        };
    }
    Ok(())
}

fn run(context: &mut RenderContext) -> Result<(), String> {
    let content = std::fs::read_to_string("src/main.rs")
        .map_err(|e| e.to_string())?;

    let mut lines: Vec<String>        = vec!();
    let mut faces: Vec<Vec<FontFace>> = vec!();

    for line in content.lines() {
        lines.push(line.to_string());
        faces.push(vec![Default::default(); line.len()]);
    }

    let mut content = TextContent {
        faces: faces,
        lines: lines,
    };

    let mut minor_modes: Vec<Box<dyn TextMinorMode>> = vec!(
        Box::new(RustMode::new()),
    );

    for minor_mode in &mut minor_modes {
        minor_mode.modify(&mut content);
    }

    // TODO: Move loop outta here!
    'mainloop: loop {
        for event in context.sdl.event_pump()?.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        context.canvas.set_draw_color(Color::RGBA(0, 0, 0, 0));
        context.canvas.clear();

        draw_content(context, &content)?;

        context.canvas.present();
    }

    Ok(())
}

pub struct TextMode {
}

impl MajorMode for TextMode  {
    fn draw(&mut self, context: &mut RenderContext) -> Result<(), String> {
        run(context)
    }
}

pub struct RustMode {
    ts_parser: Parser,
}

impl RustMode {
    fn new() -> RustMode {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language()).expect("Error loading Rust grammar");


        RustMode {
            ts_parser: parser
        }
    }
} // end impl RustMode

impl TextMinorMode for RustMode {
    // TODO: Use an "on change" hook
    fn modify(&mut self, content: &mut TextContent) {
        let tree = self.ts_parser.parse_with(&mut |_byte: usize, position: Point| -> &[u8] {
            let row = position.row as usize;
            let column = position.column as usize;
            if row < content.lines.len() {
                if column < content.lines[row].as_bytes().len() {
                    &content.lines[row].as_bytes()[column..]
                } else {
                    "\n".as_bytes()
                }
            } else {
                &[]
            }
        }, None).unwrap();

        let highlight_query = Query::new(tree_sitter_rust::language(), tree_sitter_rust::HIGHLIGHT_QUERY).unwrap();
        let mut cursor = QueryCursor::new();

        let text_callback = |node: Node| {
            let start = node.start_position();
            let end = node.end_position();

            content.lines[start.row][start.column..end.column].as_bytes()
        };

        // FIXME: Hideous for now
        let mut face_changes: Vec<(usize, usize, FontFace)> = vec!();

        for m in cursor.matches(&highlight_query, tree.root_node(), text_callback) {
            for qc in m.captures {
                let node = qc.node;
                let start_pos = node.start_position();
                let end_pos = node.end_position();

                for col in start_pos.column..end_pos.column {
                    if m.pattern_index == 47 {
                        face_changes.push((start_pos.row, col, FontFace {
                            fg_color: FontColor::Rgb(255, 100, 100),
                            bg_color: FontColor::Default,
                        }));
                    }
                }
            }
        }

        for (row, col, face) in face_changes {
            content.faces[row][col] = face;
        }
    }
}
