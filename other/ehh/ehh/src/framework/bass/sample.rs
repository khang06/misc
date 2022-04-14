use std::{cell::RefCell, ffi::c_void, rc::Rc, time::Instant};

use super::{Bass, BassChannel, BassChannelCommon, BassDrop, BassMixer};

impl Bass {
    pub fn create_sample_from_file(
        &self,
        mixer: Option<Rc<BassMixer>>,
        path: &str,
        max_chans: u32,
        flags: u32,
    ) -> Result<Rc<BassSample>, i32> {
        assert!(max_chans > 0);

        let mut encoded: Vec<u16> = path.encode_utf16().collect();
        encoded.push(0);
        let handle = bass_sys::BASS_SampleLoad(
            0,
            encoded.as_mut_ptr() as *const c_void,
            0,
            0,
            max_chans,
            flags | bass_sys::BASS_UNICODE,
        );
        if handle != 0 {
            // BASS_SAMCHAN_STREAM doesn't respect max channels, so its functionality has to be imitated
            let mixer_data = mixer.map(|mixer| {
                let mut ret = Vec::with_capacity(max_chans as usize);
                for _ in 0..max_chans {
                    let channel = bass_sys::BASS_SampleGetChannel(
                        handle,
                        bass_sys::BASS_SAMCHAN_STREAM | bass_sys::BASS_STREAM_DECODE,
                    );
                    assert!(channel != 0);

                    let channel = Rc::new(BassChannel {
                        bassdrop: self.bassdrop.clone(),
                        handle: channel,
                        autofree: false,
                    });

                    mixer
                        .add_channel(
                            channel.clone(),
                            bassmix_sys::BASS_MIXER_CHAN_NORAMPIN
                                | bassmix_sys::BASS_MIXER_CHAN_PAUSE,
                        )
                        .unwrap();

                    ret.push((channel, Instant::now()));
                }

                RefCell::new(SampleMixerData {
                    mixer: mixer.clone(),
                    streams: ret,
                })
            });

            Ok(Rc::new(BassSample {
                bassdrop: self.bassdrop.clone(),
                handle,
                mixer_data,
                owned: Default::default(),
            }))
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }
}

struct SampleMixerData {
    mixer: Rc<BassMixer>,
    streams: Vec<(Rc<BassChannel>, Instant)>,
}

pub struct BassSample {
    pub(super) bassdrop: Rc<BassDrop>,
    pub(super) handle: bass_sys::HSTREAM,
    mixer_data: Option<RefCell<SampleMixerData>>,
    owned: RefCell<Vec<Rc<BassChannel>>>,
}

impl BassSample {
    pub fn get_channel(&self, flags: u32, autofree: bool) -> Result<Rc<BassChannel>, i32> {
        let handle = bass_sys::BASS_SampleGetChannel(self.handle, flags);
        if handle != 0 {
            let ret = Rc::new(BassChannel {
                bassdrop: self.bassdrop.clone(),
                handle,
                autofree,
            });
            if !ret.autofree {
                let mut owned_ref = self.owned.borrow_mut();
                owned_ref.push(ret.clone());
            }
            Ok(ret)
        } else {
            Err(bass_sys::BASS_ErrorGetCode())
        }
    }

    pub fn play_mixer(&self, pan: f32, vol: f32) {
        // overwrites the channel that was last played the longest ago
        // doesn't prioritize channels that aren't playing, but no channel gets ended early outside of this so it shouldn't matter
        let mut mixer_data = self.mixer_data.as_ref().unwrap().borrow_mut();
        let idx = mixer_data
            .streams
            .iter()
            .enumerate()
            .reduce(|x, y| if x.1 .1 <= y.1 .1 { x } else { y })
            .unwrap()
            .0;
        mixer_data.streams[idx].1 = Instant::now();
        mixer_data
            .mixer
            .pause_channel(mixer_data.streams[idx].0.clone());
        mixer_data.streams[idx]
            .0
            .set_attrib(bass_sys::BASS_ATTRIB_PAN, pan);
        mixer_data.streams[idx]
            .0
            .set_attrib(bass_sys::BASS_ATTRIB_VOL, vol);
        mixer_data.streams[idx].0.set_mixer_position(0.0);
        mixer_data
            .mixer
            .resume_channel(mixer_data.streams[idx].0.clone());
    }
}

impl Drop for BassSample {
    fn drop(&mut self) {
        {
            let mut owned_ref = self.owned.borrow_mut();
            owned_ref.clear();
        }
        // it doesn't really matter if this fails
        // freeing an invalid handle doesn't cause memory corruption
        bass_sys::BASS_SampleFree(self.handle);
    }
}
