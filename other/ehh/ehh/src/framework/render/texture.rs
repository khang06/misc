use std::{
    ffi::c_void,
    rc::Rc,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

use gl::types::*;

use crate::math::{Rect, Vector2};

use super::{format_to_bpp, format_to_internal, util::vertical_flip_texture};

pub struct Texture2D {
    pub id: GLuint,
    pub handle: u64,
    pub width: u32,
    pub height: u32,
    pub format: GLenum,
}

impl PartialEq for Texture2D {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Drop for Texture2D {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}

impl Texture2D {
    // TODO: error checks
    pub fn new(
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        format: GLenum,
    ) -> Result<Texture2D, String> {
        unsafe {
            if let Some(data) = data {
                let bpp = format_to_bpp(format);
                if data.len() != (width * height * bpp) as usize {
                    return Err("Texture is the wrong size".to_string());
                }
            }

            let mut id: GLuint = 0;
            gl::CreateTextures(gl::TEXTURE_2D, 1, &mut id);

            gl::TextureParameteri(id, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TextureParameteri(id, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TextureParameteri(id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TextureStorage2D(id, 1, format, width as i32, height as i32);

            let handle = gl::GetTextureHandleARB(id);
            gl::MakeTextureHandleResidentARB(handle);

            let tex = Texture2D {
                id,
                handle,
                width,
                height,
                format,
            };
            if let Some(data) = data {
                tex.subimage(0, 0, data, width, height, false);
            }

            Ok(tex)
        }
    }

    pub fn subimage(&self, x: u32, y: u32, data: &[u8], width: u32, height: u32, flip: bool) {
        assert_eq!(
            data.len(),
            (width * height * format_to_bpp(self.format)) as usize
        );

        let mut temp_data = Vec::from(data);
        if flip {
            vertical_flip_texture(&mut temp_data, width as usize, height as usize, 4);
        }

        unsafe {
            gl::TextureSubImage2D(
                self.id,
                0,
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                self.format,
                gl::UNSIGNED_BYTE,
                temp_data.as_ptr() as *const c_void,
            );
        }
    }
}

// yes, i am cheating the borrow checker with this...
pub struct Texture2DArray {
    pub id: AtomicU32,
    pub handle: AtomicU64,
    pub width: u32,
    pub height: u32,
    pub layers: AtomicU32,
    pub format: GLenum,
    pub ty: GLenum,
}

impl Texture2DArray {
    // TODO: error checks...
    pub fn new(
        width: u32,
        height: u32,
        layers: u32,
        format: GLenum,
        ty: GLenum,
    ) -> Result<Texture2DArray, String> {
        unsafe {
            let mut id: GLuint = 0;
            gl::CreateTextures(gl::TEXTURE_2D_ARRAY, 1, &mut id);

            gl::TextureParameteri(id, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TextureParameteri(id, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TextureParameteri(id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TextureStorage3D(
                id,
                1,
                format_to_internal(format),
                width as i32,
                height as i32,
                layers as i32,
            );

            let handle = gl::GetTextureHandleARB(id);
            gl::MakeTextureHandleResidentARB(handle);

            Ok(Texture2DArray {
                id: AtomicU32::new(id),
                handle: AtomicU64::new(handle),
                width,
                height,
                layers: AtomicU32::new(layers),
                format,
                ty,
            })
        }
    }

    // single layer for textures that are too large for an atlas
    pub fn from_memory(
        data: &[u8],
        width: u32,
        height: u32,
        format: GLenum,
        flip: bool,
    ) -> Result<Texture2DArray, String> {
        let tex = Texture2DArray::new(width, height, 1, format, gl::UNSIGNED_BYTE)?;
        tex.subimage(0, 0, 0, data, width, height, flip);
        Ok(tex)
    }

    // TODO: do i need support for other formats here?
    #[allow(clippy::too_many_arguments)]
    pub fn subimage(
        &self,
        x: u32,
        y: u32,
        layer: u32,
        data: &[u8],
        width: u32,
        height: u32,
        flip: bool,
    ) {
        assert_eq!(
            data.len(),
            (width * height * format_to_bpp(self.format)) as usize
        );

        let mut temp_data = Vec::from(data);
        if flip {
            vertical_flip_texture(
                &mut temp_data,
                width as usize,
                height as usize,
                format_to_bpp(self.format) as usize,
            );
        }

        unsafe {
            gl::TextureSubImage3D(
                self.id.load(Ordering::Relaxed),
                0,
                x as i32,
                y as i32,
                layer as i32,
                width as i32,
                height as i32,
                1,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                temp_data.as_ptr() as *const c_void,
            );
        }
    }

    // TODO: duplicated code
    pub fn add_layer(&self) {
        unsafe {
            self.layers.fetch_add(1, Ordering::Relaxed);

            let mut new_id: GLuint = 0;
            gl::CreateTextures(gl::TEXTURE_2D_ARRAY, 1, &mut new_id);

            gl::TextureParameteri(new_id, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TextureParameteri(new_id, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TextureParameteri(new_id, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(new_id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            gl::TextureStorage3D(
                new_id,
                1,
                format_to_internal(self.format),
                self.width as i32,
                self.height as i32,
                self.layers.load(Ordering::Relaxed) as i32,
            );

            let handle = gl::GetTextureHandleARB(new_id);
            gl::MakeTextureHandleResidentARB(handle);

            gl::CopyImageSubData(
                self.id.load(Ordering::Relaxed),
                gl::TEXTURE_2D_ARRAY,
                0,
                0,
                0,
                0,
                new_id,
                gl::TEXTURE_2D_ARRAY,
                0,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                self.layers.load(Ordering::Relaxed) as i32 - 1,
            );

            gl::DeleteTextures(1, &self.id.load(Ordering::Relaxed));
            self.id.swap(new_id, Ordering::Relaxed);
            self.handle.swap(handle, Ordering::Relaxed);
        }
    }
}

impl PartialEq for Texture2DArray {
    fn eq(&self, other: &Self) -> bool {
        self.id.load(Ordering::Relaxed) == other.id.load(Ordering::Relaxed)
    }
}

impl Drop for Texture2DArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id.load(Ordering::Relaxed));
        }
    }
}

#[derive(Clone)]
pub struct TextureRegion {
    pub tex: Rc<Texture2DArray>,
    pub layer: u32,
    pub uvs: [Vector2; 2],
    pub dpi_scale: f32,
    pub width: f32,
    pub height: f32,
}

// TODO: okay this sucks ass
#[derive(Clone, Copy)]
pub struct TextureRegionWithoutTexture {
    pub layer: u32,
    pub uvs: [Vector2; 2],
    pub dpi_scale: f32,
    pub width: f32,
    pub height: f32,
}

// float imprecision bullshit makes the uvs wobble around
const PAD_SIZE: u32 = 2;

pub struct TextureAtlas {
    pub tex: Rc<Texture2DArray>,
    pub size: u32,
    pub cur_layer: u32,
    pub empty_areas: Vec<Rect>,
    pub format: GLenum,
}

impl TextureAtlas {
    pub fn new(size: u32, format: GLenum) -> TextureAtlas {
        TextureAtlas {
            tex: Rc::new(
                Texture2DArray::new(size, size, 1, format, gl::UNSIGNED_BYTE)
                    .expect("Failed to create atlas inner texture"),
            ),
            size,
            cur_layer: 0,
            empty_areas: vec![Rect::new(0, 0, size, size)],
            format,
        }
    }

    pub fn add(
        &mut self,
        name: &str,
        data: &[u8],
        width: u32,
        height: u32,
        dpi_scale: f32,
    ) -> Rc<TextureRegion> {
        if width + PAD_SIZE >= self.size || height + PAD_SIZE >= self.size {
            log::warn!(
                "{} was too big for the atlas (atlas: {}x{}, texture: {}x{})",
                name,
                self.size,
                self.size,
                width,
                height
            );

            return Rc::new(TextureRegion {
                tex: Rc::new(
                    Texture2DArray::from_memory(data, width, height, self.format, false)
                        .expect("Failed to create texture"),
                ),
                layer: 0,
                uvs: [Vector2::new(0.0, 0.0), Vector2::new(1.0, 1.0)],
                dpi_scale,
                width: width as f32,
                height: height as f32,
            });
        }

        assert!(!self.empty_areas.is_empty());
        let mut smallest_rect: Option<&mut Rect> = None;
        for x in self.empty_areas.iter_mut() {
            if x.width() >= width + PAD_SIZE
                && x.height() >= height + PAD_SIZE
                && (smallest_rect.is_none() || x.area() < smallest_rect.as_ref().unwrap().area())
            {
                smallest_rect = Some(x);
            }
        }

        if let Some(rect) = smallest_rect {
            self.tex.subimage(
                rect.left,
                rect.top,
                self.cur_layer,
                data,
                width,
                height,
                false,
            );

            let fsize = self.size as f32;
            let uv1 = Vector2::new(rect.left as f32 / fsize, rect.top as f32 / fsize);
            let uv2 = uv1 + Vector2::new(width as f32 / fsize, height as f32 / fsize);

            let r1 = Rect::new(
                rect.left + width + PAD_SIZE,
                rect.top,
                rect.right,
                rect.top + height + PAD_SIZE,
            );
            let r2 = Rect::new(
                rect.left,
                rect.top + height + PAD_SIZE,
                rect.right,
                rect.bottom,
            );

            if !r1.is_empty() {
                *rect = r1;
            } else if !r2.is_empty() {
                *rect = r2;
            } else {
                *rect = Rect::new(0, 0, 0, 0);
            }

            if !r1.is_empty() && !r2.is_empty() {
                self.empty_areas.push(r2);
            }

            return Rc::new(TextureRegion {
                tex: self.tex.clone(),
                layer: self.cur_layer,
                uvs: [uv1, uv2],
                dpi_scale,
                width: width as f32,
                height: height as f32,
            });
        }

        self.cur_layer += 1;
        self.tex.add_layer();
        self.empty_areas = vec![Rect::new(0, 0, self.size, self.size)];
        self.add(name, data, width, height, dpi_scale)
    }
}
