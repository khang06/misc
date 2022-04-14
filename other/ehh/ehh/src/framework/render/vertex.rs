use super::{VertexArray, VertexBuffer};

pub trait Vertex {
    fn setup_vertex_attrib(vao: &VertexArray, vbo: &VertexBuffer);
}

#[repr(C)]
pub struct BasicVertex {
    pub position: (f32, f32),
}

impl Vertex for BasicVertex {
    fn setup_vertex_attrib(vao: &VertexArray, vbo: &VertexBuffer) {
        unsafe {
            vbo.bind();
            vao.bind();

            // position
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<BasicVertex>() as i32,
                std::ptr::null(),
            );
        }
    }
}
