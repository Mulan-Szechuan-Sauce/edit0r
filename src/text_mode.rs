use std::collections::HashMap;

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

pub struct Global {
    faces: Faces,
}

pub struct Faces {
    theme_face_ids: Vec<usize>,
    faces: Vec<Face>,
    // Maps face names to face ids (to lookup in faces)
    face_ids: HashMap<String, usize>,
}

impl Faces {
    // Returns the face ID
    pub fn put_face(&mut self, name: String, face: Face) -> usize {
        match self.face_ids.get(&name) {
            Some(&id) => {
                self.faces[id] = face;
                id
            },
            None => {
                let id = self.faces.len();
                self.faces.push(face);
                self.face_ids.insert(name, id);
                id
            },
        }
    }

    pub fn get_face_by_name(&self, name: &String) -> Option<&Face> {
        self.face_ids.get(name).map(|&id| &self.faces[id])
    }

    pub fn get_face_by_id(&self, id: usize) -> Option<&Face> {
        self.faces.get(id)
    }

    pub fn get_face_id(&self, name: &String) -> Option<usize> {
        self.face_ids.get(name).map(|&id| id)
    }

    fn unload_theme_faces(&mut self) {
        let theme_face_ids = &self.theme_face_ids;
        let face_ids = &mut self.face_ids;
        face_ids.retain(|_, v| !theme_face_ids.contains(v));
    }

    pub fn load_theme_faces(&mut self, theme: Vec<(String, Face)>) {
        self.unload_theme_faces();

        let mut i = 0;
        for (name, face) in theme {
            if i < self.theme_face_ids.len() {
                let id = self.theme_face_ids[i];
                self.face_ids.insert(name.to_string(), id);
                self.faces[id] = face;
            } else {
                let id = self.put_face(name, face);
                self.theme_face_ids.push(id);
            }
            i += 1;
        }
    }
}

pub enum FaceColor {
    Rgb(u8, u8, u8),
}

pub struct Face {
    bg: FaceColor,
    fg: FaceColor,
}

impl Default for Face {
    fn default() -> Face {
        Face {
            bg: FaceColor::Rgb(0, 0, 0),
            fg: FaceColor::Rgb(255, 255, 255),
        }
    }
}




pub trait TextMinorMode {
    fn modify(&mut self, global: &mut Global, lines: &mut TextContent);
}

// TODO: Add margins
pub struct TextContent {
    // Dumb character by character face mapping
    // usize is the face id
    faces: Vec<Vec<usize>>,
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
    x_offset: usize,
    y_offset: u32,
    face: &Face,
    text: &str
) -> Result<(), String> {
    let fg_color = match &face.fg {
        FaceColor::Rgb(r, g, b) => Color::RGB(*r, *g, *b),
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
    let target = rect!(x_offset as u32, y_offset, width, height);

    match &face.bg {
        FaceColor::Rgb(r, g, b) => {
            context.canvas.set_draw_color(Color::RGB(*r, *g, *b));
            context.canvas.fill_rect(target)?;
        },
    };

    context.canvas.copy(&texture, None, Some(target))?;
    Ok(())
}

// Returns the height of the rendered line
fn draw_line(
    context: &mut RenderContext,
    global: &Global,
    y_offset: u32,
    char_faces: &Vec<usize>,
    line: &String,
) -> Result<u32, String> {
    let invalid_face = Face {
        bg: FaceColor::Rgb(255, 0, 0),
        fg: FaceColor::Rgb(255, 255, 255),
    };
    let mut current_face_id = usize::MAX;
    let mut current_face: &Face = &Default::default();

    if line.len() != char_faces.len() {
        panic!("Line length must equal face length");
    }

    let mut segment_start: usize = 0;
    let mut segment_len: usize = 0;

    let (char_width, char_height) = context.font.size_of_char('a').unwrap();

    for col in 0..line.len() {
        let char_face_id = char_faces[col];
        segment_len += 1;

        if char_face_id != current_face_id {
            draw_segment(
                context,
                segment_start * (char_width as usize),
                y_offset,
                current_face,
                &line[segment_start..segment_start + segment_len])?;

            segment_len = 0;
            segment_start = col;
            current_face_id = char_face_id;
            current_face = match global.faces.get_face_by_id(current_face_id) {
                Some(face) => face,
                None       => &invalid_face,
            };
        }
    }

    if segment_len > 0 {
        draw_segment(
            context,
            segment_start * (char_width as usize),
            y_offset,
            current_face,
            &line[segment_start..])?;
    }

    Ok(char_height)
}

fn draw_content(context: &mut RenderContext, global: &Global, content: &TextContent) -> Result<(), String> {
    let mut y_offset = 0;
    for i in 0..content.lines.len() {
        let line = &content.lines[i];
        y_offset += match content.faces.get(i) {
            Some(faces) => draw_line(context, global, y_offset, faces, line)?,
            None        => draw_line(context, global, y_offset, &vec!(), line)?,
        };
    }
    Ok(())
}

fn run(context: &mut RenderContext) -> Result<(), String> {
    let mut global = Global {
        faces: Faces {
            theme_face_ids: vec!(),
            faces: vec!(),
            // Maps face names to face ids (to lookup in faces)
            face_ids: HashMap::new(),
        },
    };

    global.faces.put_face("default".to_string(), Face {
        bg: FaceColor::Rgb(0, 0, 0),
        fg: FaceColor::Rgb(255, 255, 255),
    });

    let theme = vec!(
        ("keyword".to_string(), Face {
            bg: FaceColor::Rgb(0, 0, 0),
            fg: FaceColor::Rgb(255, 0, 0),
        }),
        ("function".to_string(), Face {
            bg: FaceColor::Rgb(0, 0, 0),
            fg: FaceColor::Rgb(0, 255, 0),
        }),
        ("comment".to_string(), Face {
            bg: FaceColor::Rgb(0, 0, 0),
            fg: FaceColor::Rgb(150, 150, 150),
        }),
    );

    global.faces.load_theme_faces(theme);


    let content = std::fs::read_to_string("src/main.rs")
        .map_err(|e| e.to_string())?;

    let mut lines: Vec<String>     = vec!();
    let mut faces: Vec<Vec<usize>> = vec!();

    for line in content.lines() {
        lines.push(line.to_string());
        faces.push(vec![0; line.len()]);
    }

    let mut content = TextContent {
        faces: faces,
        lines: lines,
    };

    let mut minor_modes: Vec<Box<dyn TextMinorMode>> = vec!(
        Box::new(RustMode::new()),
    );

    for minor_mode in &mut minor_modes {
        minor_mode.modify(&mut global, &mut content);
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

        draw_content(context, &global, &content)?;

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
    fn modify(&mut self, global: &mut Global, content: &mut TextContent) {
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

        let highlight_query = Query::new(
            tree_sitter_rust::language(),
            tree_sitter_rust::HIGHLIGHT_QUERY
        ).unwrap();
        let mut cursor = QueryCursor::new();

        let lines = &content.lines;

        let text_callback = |node: Node| {
            let start = node.start_position();
            let end = node.end_position();

            lines[start.row][start.column..end.column].as_bytes()
        };

        let mut ts_id_to_face_id = HashMap::<usize, usize>::new();

        for (id, name) in highlight_query.capture_names().iter().enumerate() {
            let maybe_face_id = match name.as_str() {
                "keyword"         => global.faces.get_face_id(&"keyword".to_string()),
                "function"        => global.faces.get_face_id(&"function".to_string()),
                "function.method" => global.faces.get_face_id(&"function".to_string()),
                "function.macro"  => global.faces.get_face_id(&"function".to_string()),
                "comment"         => global.faces.get_face_id(&"comment".to_string()),
                _ => None,
            };
            // 0 is magic number for default font face
            let face_id = maybe_face_id.unwrap_or(0);
            ts_id_to_face_id.insert(id, face_id);

            println!("id: {}, name: {}", id, name)
        }

        for m in cursor.matches(&highlight_query, tree.root_node(), text_callback) {
            for capture in m.captures {
                let ts_id = capture.index as usize;
                let face_id = *ts_id_to_face_id.get(&ts_id).unwrap();

                let node = capture.node;
                let start_pos = node.start_position();
                let end_pos = node.end_position();

                let row = start_pos.row;

                for col in start_pos.column..end_pos.column {
                    content.faces[row][col] = face_id;
                }
            }
        }
    }
}

/*
base_mode
-> text_mode
  -> "abstract" tree_sitter_mode
    -> rust_mode
    -> b_mode
  -> irc_mode
  -> slack_mode


put_word(string, Face, row, col)
*/
