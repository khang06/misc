use std::ffi::c_void;

use gl::types::*;

use super::{BasicVertex, Vertex, VertexArray, VertexBuffer};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpriteInstance {
    pub tex_handle: u64,
    pub layer: f32,
    pub color: u32,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub rot: f32,
    pub uv: [(f32, f32); 2],
}

impl SpriteInstance {
    pub fn setup_vertex_attrib(vao: &VertexArray, quad_vbo: &VertexBuffer, instance_vbo: GLuint) {
        BasicVertex::setup_vertex_attrib(vao, quad_vbo);

        //instance_vbo.bind();
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, instance_vbo);
        }
        vao.bind();

        unsafe {
            let mut cur_attrib = 1;
            let mut cur_offset = 0;

            // tex_handle
            gl::EnableVertexAttribArray(cur_attrib);
            gl::VertexAttribLPointer(
                cur_attrib,
                1,
                gl::UNSIGNED_INT64_ARB,
                std::mem::size_of::<SpriteInstance>() as i32,
                cur_offset as *const c_void,
            );
            gl::VertexAttribDivisor(cur_attrib, 1);
            cur_attrib += 1;
            cur_offset += 8;

            // layer
            gl::EnableVertexAttribArray(cur_attrib);
            gl::VertexAttribPointer(
                cur_attrib,
                1,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<SpriteInstance>() as i32,
                cur_offset as *const c_void,
            );
            gl::VertexAttribDivisor(cur_attrib, 1);
            cur_attrib += 1;
            cur_offset += 4;

            // color
            gl::EnableVertexAttribArray(cur_attrib);
            gl::VertexAttribIPointer(
                cur_attrib,
                1,
                gl::UNSIGNED_INT,
                std::mem::size_of::<SpriteInstance>() as i32,
                cur_offset as *const c_void,
            );
            gl::VertexAttribDivisor(cur_attrib, 1);
            cur_attrib += 1;
            cur_offset += 4;

            // position
            gl::EnableVertexAttribArray(cur_attrib);
            gl::VertexAttribPointer(
                cur_attrib,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<SpriteInstance>() as i32,
                cur_offset as *const c_void,
            );
            gl::VertexAttribDivisor(cur_attrib, 1);
            cur_attrib += 1;
            cur_offset += 8;

            // size
            gl::EnableVertexAttribArray(cur_attrib);
            gl::VertexAttribPointer(
                cur_attrib,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<SpriteInstance>() as i32,
                cur_offset as *const c_void,
            );
            gl::VertexAttribDivisor(cur_attrib, 1);
            cur_attrib += 1;
            cur_offset += 8;

            // rot
            gl::EnableVertexAttribArray(cur_attrib);
            gl::VertexAttribPointer(
                cur_attrib,
                1,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<SpriteInstance>() as i32,
                cur_offset as *const c_void,
            );
            gl::VertexAttribDivisor(cur_attrib, 1);
            cur_attrib += 1;
            cur_offset += 4;

            // uv[4]
            for _ in 0..4 {
                gl::EnableVertexAttribArray(cur_attrib);
                gl::VertexAttribPointer(
                    cur_attrib,
                    1,
                    gl::FLOAT,
                    gl::FALSE,
                    std::mem::size_of::<SpriteInstance>() as i32,
                    cur_offset as *const c_void,
                );
                gl::VertexAttribDivisor(cur_attrib, 1);
                cur_attrib += 1;
                cur_offset += 4;
            }
        }
    }
}
