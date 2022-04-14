use std::{collections::HashMap, rc::Rc};

use crate::framework::render::{TextureAtlas, TextureRegion};

use super::skin::Skin;

pub struct AnimatedTexture {
    textures: Vec<Rc<TextureRegion>>,
    frame_delay: f32,
    pub width: f32,
    pub height: f32,
    pub dpi_scale: f32,
}

impl AnimatedTexture {
    pub fn new(textures: &[Rc<TextureRegion>], framerate: f32) -> AnimatedTexture {
        assert!(!textures.is_empty());
        AnimatedTexture {
            textures: Vec::from(textures),
            frame_delay: if framerate > 0.0 {
                1000.0 / framerate
            } else {
                1000.0 / textures.len() as f32
            },
            // no sane person makes an animation that changes size...
            width: textures[0].width,
            height: textures[0].height,
            dpi_scale: textures[0].dpi_scale,
        }
    }

    pub fn get_tex(&self, start_time: i32, cur_time: i32) -> Rc<TextureRegion> {
        if self.textures.len() == 1 {
            return self.textures[0].clone();
        }

        self.textures
            [((cur_time - start_time) as f32 / self.frame_delay) as usize % self.textures.len()]
        .clone()
    }
}

pub struct AssetLoader {
    pub skin: Skin,
    // TODO: proper layering system once ui and stuff get implemented
    // also, i would need to store textures in cpu memory to recreate the atlas between beatmap/skin changes
    atlas: TextureAtlas,
    tex_map: HashMap<String, Rc<AnimatedTexture>>,
}

impl AssetLoader {
    pub fn new(skin_path: &str) -> AssetLoader {
        let mut loader = AssetLoader {
            skin: Skin::new(skin_path),
            atlas: TextureAtlas::new(4096, gl::RGBA),
            tex_map: Default::default(),
        };

        // cache common stuff preemptively
        loader.lookup_tex("cursor");
        loader.lookup_tex("approachcircle");
        loader.lookup_tex("hitcircle");
        loader.lookup_tex("hitcircleoverlay");
        loader.lookup_tex("sliderscorepoint");
        loader.lookup_tex("reversearrow");
        loader.lookup_anim("sliderb", false);

        loader
    }

    fn lookup_internal(
        &mut self,
        name: &str,
        animated: bool,
        has_dash: bool,
    ) -> Rc<AnimatedTexture> {
        if let Some(tex) = self.tex_map.get(name) {
            tex.clone()
        } else if let Some(tex) = self
            .skin
            .try_load_tex(&mut self.atlas, name, animated, has_dash)
        {
            self.tex_map.insert(name.to_string(), tex.clone());
            tex
        } else {
            // TODO: generate a placeholder
            panic!("Failed to load texture {}", name);
        }
    }

    pub fn lookup_anim(&mut self, name: &str, has_dash: bool) -> Rc<AnimatedTexture> {
        self.lookup_internal(name, true, has_dash)
    }

    pub fn lookup_tex(&mut self, name: &str) -> Rc<TextureRegion> {
        let anim = self.lookup_internal(name, false, false);
        assert!(!anim.textures.is_empty());
        anim.textures[0].clone()
    }
}
