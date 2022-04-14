// for shared behavior between HCHANNEL and HSTREAM
pub trait BassChannelCommon {
    fn get_handle(&self) -> u32;

    fn play(&self, restart: bool) -> Result<(), i32> {
        if bass_sys::BASS_ChannelPlay(self.get_handle(), restart as i32) != 0 {
            Ok(())
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }
    fn pause(&self) -> Result<(), i32> {
        if bass_sys::BASS_ChannelPause(self.get_handle()) != 0 {
            Ok(())
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }

    fn get_mixer_position(&self) -> f64 {
        let bytes =
            bassmix_sys::BASS_Mixer_ChannelGetPosition(self.get_handle(), bass_sys::BASS_POS_BYTE);
        if bytes == u64::MAX {
            panic!(
                "Something went wrong with BASS_Mixer_ChannelGetPosition: {}",
                bass_sys::BASS_ErrorGetCode()
            );
        }
        bass_sys::BASS_ChannelBytes2Seconds(self.get_handle(), bytes) * 1000.0
    }
    fn set_mixer_position(&self, pos: f64) -> bool {
        let bytes = bass_sys::BASS_ChannelSeconds2Bytes(self.get_handle(), pos.max(0.0) / 1000.0);
        bassmix_sys::BASS_Mixer_ChannelSetPosition(
            self.get_handle(),
            bytes,
            bass_sys::BASS_POS_BYTE,
        ) && pos >= 0.0
    }

    fn get_mixer_is_active(&self) -> bool {
        // TODO: is this correct?
        bassmix_sys::BASS_Mixer_ChannelIsActive(self.get_handle()) == bass_sys::BASS_ACTIVE_PLAYING
    }

    fn get_attrib(&self, attrib: u32) -> Option<f32> {
        let mut ret: f32 = 0.0;
        if bass_sys::BASS_ChannelGetAttribute(self.get_handle(), attrib, &mut ret) != 0 {
            Some(ret)
        } else {
            None
        }
    }
    fn set_attrib(&self, attrib: u32, value: f32) -> Option<()> {
        if bass_sys::BASS_ChannelSetAttribute(self.get_handle(), attrib, value) != 0 {
            Some(())
        } else {
            None
        }
    }

    fn get_device(&self) -> Result<u32, i32> {
        let res = bass_sys::BASS_ChannelGetDevice(self.get_handle());
        if res != u32::MAX {
            Ok(res)
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }
    fn set_device(&self, device: u32) -> Result<(), i32> {
        if bass_sys::BASS_ChannelSetDevice(self.get_handle(), device) != 0 {
            Ok(())
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }
}
