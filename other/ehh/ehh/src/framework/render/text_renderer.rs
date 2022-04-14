use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::math::Vector2;

use super::{
    DrawBatch, DrawBatchCommand, TextureAtlas, TextureRegion, TextureRegionWithoutTexture,
};

// https://learnopengl.com/In-Practice/Text-Rendering
#[derive(Clone, Copy)]
pub struct TextRendererChar {
    tex: TextureRegionWithoutTexture,
    size: (f32, f32),
    bearing: (f32, f32),
    advance: i32,
}

pub struct TextRenderer {
    freetype: freetype::Library,
    atlas: TextureAtlas,
    fonts: Vec<freetype::Face>, // ordered by decreasing priority
    chars: HashMap<char, TextRendererChar>,
}

impl TextRenderer {
    pub fn new(font_files: &[Vec<u8>]) -> TextRenderer {
        let freetype = freetype::Library::init().expect("Failed to load Freetype");

        let mut fonts = Vec::with_capacity(font_files.len());
        for x in font_files {
            let face = freetype
                .new_memory_face(Rc::new(x.clone()), 0)
                .expect("Failed to load font");
            face.set_pixel_sizes(0, 64).unwrap();
            fonts.push(face);
        }

        let mut renderer = TextRenderer {
            freetype,
            atlas: TextureAtlas::new(1024, gl::RGBA),
            fonts,
            chars: Default::default(),
        };

        // preload common ascii characters
        for i in 32 as char..=126 as char {
            renderer.get_char(i);
        }

        renderer
    }

    pub fn get_char(&mut self, to_load: char) -> TextRendererChar {
        if let Some(loaded) = self.chars.get(&to_load) {
            return *loaded;
        }

        let mut found_font = None;
        for x in &self.fonts {
            if x.get_char_index(to_load as usize) != 0
                && x.load_char(to_load as usize, freetype::face::LoadFlag::RENDER)
                    .is_ok()
            {
                found_font = Some(x);
                break;
            }
        }

        assert!(found_font.is_some());
        let found_font = found_font.unwrap();

        let glyph = found_font.glyph();
        let bitmap = glyph.bitmap();
        let mut rgba_bitmap = vec![0xFF; (bitmap.width() * bitmap.rows() * 4) as usize];
        for i in 0..(bitmap.width() * bitmap.rows()) as usize {
            rgba_bitmap[i * 4 + 3] = bitmap.buffer()[i];
        }

        let tex = self.atlas.add(
            &to_load.to_string(),
            &rgba_bitmap,
            bitmap.width() as u32,
            bitmap.rows() as u32,
            1.0,
        );

        *self.chars.entry(to_load).or_insert(TextRendererChar {
            tex: TextureRegionWithoutTexture {
                layer: tex.layer,
                uvs: tex.uvs,
                dpi_scale: tex.dpi_scale,
                width: tex.width,
                height: tex.height,
            },
            size: (bitmap.width() as f32, bitmap.rows() as f32),
            bearing: (glyph.bitmap_left() as f32, glyph.bitmap_top() as f32),
            advance: glyph.advance().x,
        })
    }

    pub fn text_length(&mut self, text: &str, scale: f32) -> f32 {
        // TODO: handle multiline
        let mut ret = 0.0;
        for c in text.chars() {
            let ch = self.get_char(c);
            ret += ch.advance as f32 / 64.0 * scale;
        }
        ret
    }
}

pub struct TextSprite {
    renderer: Rc<RefCell<TextRenderer>>,
    text: String,
    x: f32,
    y: f32,
    scale: f32,
    alignment: Alignment,
    line_height: f32,
    cached: Vec<DrawBatchCommand>,
}

pub enum Alignment {
    Left,
    Right,
    Center,
}

impl TextSprite {
    pub fn new(
        renderer: Rc<RefCell<TextRenderer>>,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        alignment: Alignment,
    ) -> TextSprite {
        // TODO: how should i handle this properly instead of just using the first font
        let line_height = renderer.borrow().fonts[0].size_metrics().unwrap().height as f32 / 64.0;
        let mut sprite = TextSprite {
            renderer,
            text: text.to_string(),
            x,
            y,
            scale,
            alignment,
            line_height,
            cached: Default::default(),
        };
        sprite.refresh();

        sprite
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.refresh();
    }

    pub fn refresh(&mut self) {
        self.cached.clear();
        if self.text.is_empty() {
            return;
        }

        let mut renderer = self.renderer.borrow_mut();

        for (line_num, line) in self.text.lines().enumerate() {
            let mut x = self.x
                + match self.alignment {
                    Alignment::Left => 0.0,
                    Alignment::Center => -renderer.text_length(line, self.scale) / 2.0,
                    Alignment::Right => -renderer.text_length(line, self.scale),
                };
            let y = self.y + (line_num + 1) as f32 * self.line_height * self.scale;
            for c in line.chars() {
                let ch = renderer.get_char(c);
                let x_pos = x + (ch.size.0 / 2.0 + ch.bearing.0) * self.scale;
                let y_pos = y + (ch.size.1 / 2.0 - ch.bearing.1) * self.scale;

                self.cached.push(DrawBatchCommand {
                    tex: Rc::new(TextureRegion {
                        tex: renderer.atlas.tex.clone(),
                        layer: ch.tex.layer,
                        uvs: ch.tex.uvs,
                        dpi_scale: ch.tex.dpi_scale,
                        width: ch.size.0,
                        height: ch.size.1,
                    }),
                    pos: Vector2::new(x_pos, y_pos),
                    scale: self.scale,
                    color: 0xFFFFFFFF,
                    rot: 0.0,
                });

                x += ch.advance as f32 / 64.0 * self.scale;
            }
        }
    }

    pub fn add_to_batch(&self, batch: &mut DrawBatch) {
        batch.add_batch(&self.cached);
    }
}
