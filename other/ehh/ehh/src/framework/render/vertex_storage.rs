use gl::types::*;

pub struct BufferedVertexStorage<T> {
    pub id: GLuint,
    size: usize,
    ptr: *mut T,
    idx: usize,
    sync: Vec<GLsync>,
}

impl<T> BufferedVertexStorage<T> {
    pub fn new(size: usize, buffers: usize, flags: GLbitfield) -> BufferedVertexStorage<T> {
        unsafe {
            let mut id: GLuint = 0;
            gl::GenBuffers(1, &mut id);
            gl::BindBuffer(gl::ARRAY_BUFFER, id);

            let total_size = (size * buffers * std::mem::size_of::<T>()) as isize;
            gl::BufferStorage(gl::ARRAY_BUFFER, total_size, std::ptr::null(), flags);

            let ptr = gl::MapBufferRange(gl::ARRAY_BUFFER, 0, total_size, flags) as *mut T;
            BufferedVertexStorage {
                id,
                size,
                ptr,
                idx: 0,
                sync: vec![0 as GLsync; buffers],
            }
        }
    }

    pub fn draw(&mut self, data: &[T]) {
        if data.is_empty() {
            return;
        }
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.id);
        }
        for x in data.chunks(self.size) {
            self.draw_internal(x);
        }
    }

    fn draw_internal(&mut self, data: &[T]) {
        assert!(data.len() <= self.size);

        unsafe {
            // wait for the current buffer slice to stop being used...
            if self.sync[self.idx] != 0 as GLsync {
                loop {
                    let ret =
                        gl::ClientWaitSync(self.sync[self.idx], gl::SYNC_FLUSH_COMMANDS_BIT, 1);
                    if ret == gl::ALREADY_SIGNALED || ret == gl::CONDITION_SATISFIED {
                        break;
                    }
                }
            }

            // copy
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.ptr.add(self.size * self.idx),
                data.len(),
            );

            // send the draw command
            gl::DrawElementsInstancedBaseInstance(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::ptr::null(),
                data.len() as i32,
                (self.size * self.idx) as u32,
            );

            // set up the sync shit again
            if self.sync[self.idx] != 0 as GLsync {
                gl::DeleteSync(self.sync[self.idx]);
            }
            self.sync[self.idx] = gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0);

            // switch buffers
            self.idx = (self.idx + 1) % self.sync.len();
        }
    }
}

impl<T> Drop for BufferedVertexStorage<T> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}
