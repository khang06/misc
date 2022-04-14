use std::ffi::CStr;

use super::Bass;

// safe version of bass_sys::BassDeviceInfo
pub struct BassDeviceInfo {
    pub name: String,
    pub driver: String,
    pub flags: u32,
}

impl Bass {
    pub fn get_device(&self) -> Result<u32, i32> {
        let ret = bass_sys::BASS_GetDevice();
        if ret == u32::MAX {
            Err(bass_sys::BASS_ErrorGetCode())
        } else {
            Ok(ret)
        }
    }

    pub fn set_device(&self, id: u32) -> Result<(), i32> {
        if bass_sys::BASS_SetDevice(id) != 0 {
            Ok(())
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }

    pub fn get_device_info(&self, id: u32) -> Result<BassDeviceInfo, i32> {
        unsafe {
            let mut info: bass_sys::BassDeviceInfo = std::mem::zeroed();
            if bass_sys::BASS_GetDeviceInfo(id, &mut info) != 0 {
                Ok(BassDeviceInfo {
                    name: CStr::from_ptr(info.name as *const i8)
                        .to_str()
                        .unwrap_or("[invalid]")
                        .to_string(),
                    driver: CStr::from_ptr(info.name as *const i8)
                        .to_str()
                        .unwrap_or("[invalid]")
                        .to_string(),
                    flags: info.flags,
                })
            } else {
                Err(bass_sys::BASS_ErrorGetCode())
            }
        }
    }
}
