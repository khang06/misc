use std::ffi::c_void;

use gl::types::*;

pub struct VertexBuffer {
    pub id: GLuint,
    pub size: usize,
}

impl VertexBuffer {
    pub fn from_data<T>(data: &[T], usage: GLenum) -> VertexBuffer {
        unsafe {
            let mut id: GLuint = 0;
            gl::GenBuffers(1, &mut id);
            gl::BindBuffer(gl::ARRAY_BUFFER, id);

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (data.len() * std::mem::size_of::<T>()) as isize,
                data.as_ptr() as *const c_void,
                usage,
            );

            VertexBuffer {
                id,
                size: data.len(),
            }
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.id);
        }
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}
