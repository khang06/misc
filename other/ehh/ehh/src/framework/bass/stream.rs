use std::{ffi::c_void, rc::Rc};

use super::{channel_common::BassChannelCommon, Bass, BassDrop};

impl Bass {
    pub fn create_stream_from_file(&self, path: &str, flags: u32) -> Result<Rc<BassStream>, i32> {
        let mut encoded: Vec<u16> = path.encode_utf16().collect();
        encoded.push(0);
        let handle = bass_sys::BASS_StreamCreateFile(
            0,
            encoded.as_mut_ptr() as *const c_void,
            0,
            0,
            flags | bass_sys::BASS_UNICODE,
        );
        if handle != 0 {
            Ok(Rc::new(BassStream {
                bassdrop: self.bassdrop.clone(),
                handle,
            }))
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }
}

pub struct BassStream {
    pub(super) bassdrop: Rc<BassDrop>,
    pub(super) handle: bass_sys::HSTREAM,
}

impl BassChannelCommon for BassStream {
    fn get_handle(&self) -> u32 {
        self.handle as u32
    }
}

impl Drop for BassStream {
    fn drop(&mut self) {
        // it doesn't really matter if this fails
        // usually it means that it got autofreed
        // also, freeing an invalid handle doesn't cause memory corruption
        bass_sys::BASS_StreamFree(self.get_handle());
    }
}
