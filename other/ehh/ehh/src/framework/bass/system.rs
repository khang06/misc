// based on rust-sdl2

use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

static BASS_INITED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct Bass {
    pub(super) bassdrop: Rc<BassDrop>,
}

impl Bass {
    pub fn new(device: i32, sample_rate: u32, flags: u32) -> Result<Bass, String> {
        let was_alive = BASS_INITED.swap(true, Ordering::Relaxed);

        if was_alive {
            return Err("Can't init BASS twice!".to_string());
        }

        if bass_sys::BASS_Init(
            device,
            sample_rate,
            flags,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) != 0
        {
            Ok(Bass {
                bassdrop: Rc::new(BassDrop),
            })
        } else {
            Err(format!(
                "Failed to initialize BASS: {}",
                bass_sys::BASS_ErrorGetCode()
            ))
        }
    }

    pub fn get_version(&self) -> (u8, u8, u8, u8) {
        let raw = bass_sys::BASS_GetVersion();
        (
            ((raw >> 24) & 0xFF) as u8,
            ((raw >> 16) & 0xFF) as u8,
            ((raw >> 8) & 0xFF) as u8,
            (raw & 0xFF) as u8,
        )
    }

    pub fn get_bassmix_version(&self) -> (u8, u8, u8, u8) {
        let raw = bassmix_sys::BASS_Mixer_GetVersion();
        (
            ((raw >> 24) & 0xFF) as u8,
            ((raw >> 16) & 0xFF) as u8,
            ((raw >> 8) & 0xFF) as u8,
            (raw & 0xFF) as u8,
        )
    }

    pub fn get_config(&self, option: u32) -> Option<u32> {
        let res = bass_sys::BASS_GetConfig(option);
        if res != u32::MAX {
            Some(res)
        } else {
            None
        }
    }

    pub fn set_config(&self, option: u32, value: u32) -> Option<()> {
        if bass_sys::BASS_SetConfig(option, value) != 0 {
            Some(())
        } else {
            None
        }
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct BassDrop;

impl Drop for BassDrop {
    fn drop(&mut self) {
        let was_alive = BASS_INITED.swap(false, Ordering::Relaxed);

        if !was_alive {
            panic!("Tried to free BASS, but it wasn't initialized!");
        }

        assert!(bass_sys::BASS_Free() != 0);
    }
}
