use std::collections::HashMap;

use cgmath::Matrix;
use gl::types::*;

pub struct Shader {
    pub kind: GLenum,
    pub id: GLuint,
}

impl Shader {
    pub fn from_string(src: &str, kind: GLenum) -> Result<Shader, String> {
        unsafe {
            let id = gl::CreateShader(kind);

            let src_ptr = src.as_bytes().as_ptr() as *const i8;
            let src_len = src.len() as GLint;
            gl::ShaderSource(id, 1, &src_ptr, &src_len);
            gl::CompileShader(id);

            let mut res: GLint = 0;
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut res);

            if res != 0 {
                Ok(Shader { kind, id })
            } else {
                let mut buf_len = 0;
                gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut buf_len);

                let mut buf: Vec<u8> = vec![0; buf_len as usize];
                let buf_ptr = buf.as_mut_ptr() as *mut gl::types::GLchar;
                gl::GetShaderInfoLog(id, buf_len, std::ptr::null_mut(), buf_ptr);

                Err(String::from_utf8_lossy(&buf).to_string())
            }
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

pub struct InputAttribute {
    pub size: usize,
    pub gl_type: GLenum,
    pub location: GLint,
}

pub struct ShaderProgram {
    pub id: GLuint,
    pub uniforms: HashMap<String, InputAttribute>,
}

impl ShaderProgram {
    pub fn new(shaders: &[&Shader]) -> Result<ShaderProgram, String> {
        unsafe {
            let id = gl::CreateProgram();
            for shader in shaders {
                gl::AttachShader(id, shader.id);
            }
            gl::LinkProgram(id);

            let mut res: GLint = 1;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut res);

            if res != 0 {
                for shader in shaders {
                    gl::DetachShader(id, shader.id);
                }

                let mut ret = ShaderProgram {
                    id,
                    uniforms: Default::default(),
                };

                ret.cache_uniforms();

                Ok(ret)
            } else {
                let mut buf_len = 0;
                gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut buf_len);

                let mut buf: Vec<u8> = vec![0; buf_len as usize];
                let buf_ptr = buf.as_mut_ptr() as *mut gl::types::GLchar;
                gl::GetProgramInfoLog(id, buf_len, std::ptr::null_mut(), buf_ptr);

                Err(String::from_utf8_lossy(&buf).to_string())
            }
        }
    }

    fn cache_uniforms(&mut self) {
        unsafe {
            let mut count = 0;
            let mut max_len = 0;
            gl::GetProgramiv(self.id, gl::ACTIVE_UNIFORMS, &mut count);
            gl::GetProgramiv(self.id, gl::ACTIVE_UNIFORM_MAX_LENGTH, &mut max_len);

            for i in 0..count {
                let mut buf: Vec<u8> = vec![0; max_len as usize];
                let buf_ptr = buf.as_mut_ptr() as *mut gl::types::GLchar;

                let mut len = 0;
                let mut size = 0;
                let mut gl_type = 0;
                gl::GetActiveUniform(
                    self.id,
                    i as u32,
                    max_len,
                    &mut len,
                    &mut size,
                    &mut gl_type,
                    buf_ptr,
                );
                buf.truncate(len as usize);

                let location = gl::GetUniformLocation(self.id, buf_ptr);

                self.uniforms.insert(
                    String::from_utf8_lossy(&buf).to_string(),
                    InputAttribute {
                        size: size as usize,
                        gl_type,
                        location,
                    },
                );
            }
        }
    }

    pub fn get_uniform(&self, name: &str) -> Option<&InputAttribute> {
        self.uniforms.get(&name.to_string())
    }

    pub fn get_uniform_location(&self, name: &str) -> Option<i32> {
        self.get_uniform(name).map(|uniform| uniform.location)
    }

    pub fn set_uniform_i32(&self, name: &str, data: i32) {
        if let Some(location) = self.get_uniform_location(name) {
            unsafe {
                gl::Uniform1i(location, data);
            }
        } else {
            panic!("Couldn't find uniform named {}", name);
        }
    }

    pub fn set_uniform_matrix_4fv(&self, name: &str, data: cgmath::Matrix4<f32>) {
        if let Some(location) = self.get_uniform_location(name) {
            unsafe {
                gl::UniformMatrix4fv(location, 1, gl::FALSE, data.as_ptr() as *const f32);
            }
        } else {
            panic!("Couldn't find uniform named {}", name);
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
