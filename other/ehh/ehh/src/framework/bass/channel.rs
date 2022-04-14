use std::rc::Rc;

use super::{channel_common::BassChannelCommon, BassDrop};

pub struct BassChannel {
    pub(super) bassdrop: Rc<BassDrop>,
    pub(super) handle: bass_sys::HCHANNEL,
    pub(super) autofree: bool,
}

impl BassChannelCommon for BassChannel {
    fn get_handle(&self) -> u32 {
        self.handle as u32
    }
}

impl Drop for BassChannel {
    fn drop(&mut self) {
        if !self.autofree {
            // it doesn't really matter if this fails
            // freeing an invalid handle doesn't cause memory corruption
            bass_sys::BASS_ChannelStop(self.get_handle());
        }
    }
}
