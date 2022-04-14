use std::rc::Rc;

use rgb::ComponentBytes;

use crate::framework::render::{texture::TextureRegion, TextureAtlas};

use super::asset_loader::AnimatedTexture;

// TODO: start parsing skin.ini once i have to
pub struct Skin {
    pub base_path: String,
}

impl Skin {
    pub fn new(base_path: &str) -> Skin {
        Skin {
            base_path: base_path.to_string(),
        }
    }

    fn tex_load_internal(&self, atlas: &mut TextureAtlas, name: &str) -> Option<Rc<TextureRegion>> {
        // try to load @2x sprite first
        if let Ok(img) = lodepng::decode32_file(format!("{}/{}@2x.png", self.base_path, name)) {
            Some(atlas.add(
                name,
                img.buffer.as_bytes(),
                img.width as u32,
                img.height as u32,
                2.0,
            ))
        } else if let Ok(img) = lodepng::decode32_file(format!("{}/{}.png", self.base_path, name)) {
            Some(atlas.add(
                name,
                img.buffer.as_bytes(),
                img.width as u32,
                img.height as u32,
                1.0,
            ))
        } else {
            None
        }
    }

    pub fn try_load_tex(
        &self,
        atlas: &mut TextureAtlas,
        name: &str,
        animated: bool,
        has_dash: bool,
    ) -> Option<Rc<AnimatedTexture>> {
        // TODO: clean this up once let chains work properly
        if animated {
            let dash = if has_dash { "-" } else { "" };
            if let Some(first_tex) = self.tex_load_internal(atlas, &format!("{}{}0", dash, name)) {
                let mut textures = vec![first_tex];
                let mut idx = 1;
                while let Some(tex) =
                    self.tex_load_internal(atlas, &format!("{}{}{}", name, dash, idx))
                {
                    textures.push(tex.clone());
                    idx += 1;
                }

                // TODO: fix framerate once i start parsing skin.ini
                Some(Rc::new(AnimatedTexture::new(&textures, -1.0)))
            } else {
                self.tex_load_internal(atlas, name)
                    .map(|tex| Rc::new(AnimatedTexture::new(&[tex], -1.0)))
            }
        } else {
            self.tex_load_internal(atlas, name)
                .map(|tex| Rc::new(AnimatedTexture::new(&[tex], -1.0)))
        }
    }
}
