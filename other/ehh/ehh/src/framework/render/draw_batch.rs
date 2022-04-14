use std::{rc::Rc, sync::atomic::Ordering};

use cgmath::Matrix4;

use crate::math::Vector2;

use super::{
    texture::TextureRegion, BasicVertex, BufferedVertexStorage, ElementBuffer, Shader,
    ShaderProgram, SpriteInstance, VertexArray, VertexBuffer,
};

pub struct DrawBatch {
    queue: Vec<DrawBatchCommand>,

    program: ShaderProgram,
    quad_vbo: VertexBuffer,
    vao: VertexArray,
    ebo: ElementBuffer,
    bvs: BufferedVertexStorage<SpriteInstance>,
}

#[derive(PartialEq)]
pub enum Origin {
    // yep, this code is in 100% american cheeseburger freedom english
    Center,
    TopLeft,
}

impl DrawBatch {
    pub fn new(proj: Matrix4<f32>) -> DrawBatch {
        // TODO: having to do a relative path like this sucks
        let vert = Shader::from_string(
            include_str!("../../../assets/shaders/sprite.vert"),
            gl::VERTEX_SHADER,
        )
        .unwrap();
        let frag = Shader::from_string(
            include_str!("../../../assets/shaders/sprite.frag"),
            gl::FRAGMENT_SHADER,
        )
        .unwrap();
        let program = ShaderProgram::new(&[&vert, &frag]).unwrap();

        program.bind();
        program.set_uniform_matrix_4fv("proj", proj);

        let vertices: [BasicVertex; 4] = [
            // bottom left
            BasicVertex {
                position: (-0.5, -0.5),
            },
            // bottom right
            BasicVertex {
                position: (0.5, -0.5),
            },
            // top right
            BasicVertex {
                position: (0.5, 0.5),
            },
            // top left
            BasicVertex {
                position: (-0.5, 0.5),
            },
        ];
        let indices: [u32; 6] = [0, 1, 2, 2, 3, 0];

        let quad_vbo = VertexBuffer::from_data(&vertices, gl::STATIC_DRAW);
        let ebo = ElementBuffer::from_data(&indices, gl::STATIC_DRAW);
        let vao = VertexArray::new();
        let bvs = BufferedVertexStorage::<SpriteInstance>::new(
            8192,
            3,
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
        );
        SpriteInstance::setup_vertex_attrib(&vao, &quad_vbo, bvs.id);

        DrawBatch {
            queue: Default::default(),
            program,
            quad_vbo,
            vao,
            ebo,
            bvs,
        }
    }

    pub fn add(
        &mut self,
        tex: Rc<TextureRegion>,
        pos: Vector2,
        scale: f32,
        origin: Origin,
        color: u32,
        rot: f32,
    ) {
        // don't render invisible objects
        if color & 0xFF000000 == 0 {
            return;
        }

        let scale = scale / tex.dpi_scale;
        let offset = match origin {
            Origin::Center => Vector2::new(0.0, 0.0),
            Origin::TopLeft => Vector2::new(-tex.width / 2.0, -tex.height / 2.0),
        } * scale;
        self.queue.push(DrawBatchCommand {
            tex,
            pos: pos + offset,
            scale,
            color,
            rot,
        });
    }

    pub fn add_batch(&mut self, batch: &[DrawBatchCommand]) {
        self.queue.extend_from_slice(batch);
    }

    pub fn draw(&mut self) {
        let mut queue = Vec::with_capacity(self.queue.len());
        self.program.bind();
        self.vao.bind();
        self.ebo.bind();

        for x in self.queue.iter() {
            let tex_handle = x.tex.tex.handle.load(Ordering::Relaxed);
            queue.push(SpriteInstance {
                tex_handle,
                layer: x.tex.layer as f32,
                position: x.pos.into(),
                size: (x.tex.width * x.scale, x.tex.height * x.scale),
                uv: [x.tex.uvs[0].into(), x.tex.uvs[1].into()],
                color: x.color,
                rot: x.rot,
            });
        }

        self.bvs.draw(&queue);
        self.queue.clear();
    }
}

// position is in 1024x768 native osu area
#[derive(Clone)]
pub struct DrawBatchCommand {
    pub tex: Rc<TextureRegion>,
    pub pos: Vector2,
    pub scale: f32,
    pub color: u32,
    pub rot: f32,
}
