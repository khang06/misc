use std::ffi::c_void;

use gl::types::*;

pub struct ElementBuffer {
    pub id: GLuint,
    pub size: usize,
}

impl ElementBuffer {
    pub fn from_data(data: &[u32], usage: GLenum) -> ElementBuffer {
        unsafe {
            let mut id: GLuint = 0;
            gl::GenBuffers(1, &mut id);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, id);

            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (data.len() * std::mem::size_of::<u32>()) as isize,
                data.as_ptr() as *const c_void,
                usage,
            );

            ElementBuffer {
                id,
                size: data.len() * std::mem::size_of::<u32>(),
            }
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
        }
    }
}

impl Drop for ElementBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}
