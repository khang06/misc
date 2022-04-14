use std::{cell::RefCell, rc::Rc};

use super::{channel_common::BassChannelCommon, Bass, BassDrop};

impl Bass {
    pub fn create_mixer(&self, freq: u32, chans: u32, flags: u32) -> Result<BassMixer, i32> {
        let handle = bassmix_sys::BASS_Mixer_StreamCreate(freq, chans, flags);
        if handle != 0 {
            Ok(BassMixer {
                bassdrop: self.bassdrop.clone(),
                handle,
                owned: Default::default(),
            })
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }
}

pub struct BassMixer {
    pub(super) bassdrop: Rc<BassDrop>,
    pub(super) handle: bass_sys::HSTREAM,
    owned: RefCell<Vec<Rc<dyn BassChannelCommon>>>,
}

impl BassMixer {
    pub fn add_channel(&self, channel: Rc<dyn BassChannelCommon>, flags: u32) -> Result<(), i32> {
        if bassmix_sys::BASS_Mixer_StreamAddChannel(self.get_handle(), channel.get_handle(), flags)
            != 0
        {
            if flags & bass_sys::BASS_STREAM_AUTOFREE == 0 {
                let mut owned_ref = self.owned.borrow_mut();
                owned_ref.push(channel.clone());
            }
            Ok(())
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }

    pub fn pause_channel(&self, channel: Rc<dyn BassChannelCommon>) {
        bassmix_sys::BASS_Mixer_ChannelFlags(
            channel.get_handle(),
            bassmix_sys::BASS_MIXER_CHAN_PAUSE,
            bassmix_sys::BASS_MIXER_CHAN_PAUSE,
        );
    }

    pub fn resume_channel(&self, channel: Rc<dyn BassChannelCommon>) {
        bassmix_sys::BASS_Mixer_ChannelFlags(
            channel.get_handle(),
            0,
            bassmix_sys::BASS_MIXER_CHAN_PAUSE,
        );
    }
}

impl BassChannelCommon for BassMixer {
    fn get_handle(&self) -> u32 {
        self.handle as u32
    }
}

impl Drop for BassMixer {
    fn drop(&mut self) {
        {
            let mut owned_ref = self.owned.borrow_mut();
            owned_ref.clear();
        }
        assert!(bass_sys::BASS_StreamFree(self.get_handle()) != 0);
    }
}
