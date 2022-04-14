use gl::types::*;

pub struct VertexArray {
    pub id: GLuint,
}

impl Default for VertexArray {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexArray {
    pub fn new() -> VertexArray {
        unsafe {
            let mut id: GLuint = 0;
            gl::GenVertexArrays(1, &mut id);

            VertexArray { id }
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.id);
        }
    }
}
