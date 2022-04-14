use std::{cell::RefCell, rc::Rc};

use crate::framework::{
    bass::{Bass, BassChannelCommon, BassMixer, BassSample, BassStream},
    clock::{BassStreamClock, Clock, DecoupledClock, OffsetClock},
};

use super::asset_loader::AssetLoader;

pub struct AudioManager {
    bass: Rc<Bass>,
    asset_loader: Rc<RefCell<AssetLoader>>,
    mixer: Rc<BassMixer>,
    main_track: Rc<BassStream>,
    main_track_clock: OffsetClock,
    test_sample: Rc<BassSample>,
}

impl AudioManager {
    pub fn new(
        bass: Rc<Bass>,
        asset_loader: Rc<RefCell<AssetLoader>>,
        audio_path: &str,
    ) -> AudioManager {
        let mixer = Rc::new(
            bass.create_mixer(
                44100,
                2,
                bassmix_sys::BASS_MIXER_NONSTOP | bass_sys::BASS_SAMPLE_FLOAT,
            )
            .expect("Failed to create mixer"),
        );
        mixer.set_attrib(bass_sys::BASS_ATTRIB_BUFFER, 0.0);
        mixer.set_attrib(bass_sys::BASS_ATTRIB_VOL, 0.5);
        mixer
            .set_device(bass.get_device().unwrap())
            .expect("Failed to set mixer device");
        mixer.play(false).expect("Failed to start the mixer");

        // TODO: gracefully handle this failing
        let main_track = bass
            .create_stream_from_file(
                audio_path,
                bass_sys::BASS_STREAM_DECODE | bass_sys::BASS_STREAM_PRESCAN,
            )
            .expect("Failed to load audio track");
        mixer
            .add_channel(main_track.clone(), bassmix_sys::BASS_MIXER_CHAN_PAUSE)
            .expect("Failed to add audio track to the mixer");
        let main_track_clock = OffsetClock::new(
            Box::new(DecoupledClock::new(Box::new(BassStreamClock::new(
                mixer.clone(),
                main_track.clone(),
            )))),
            -15.0, // offset used by osu for wasapi backend
        );

        let test_sample = bass
            .create_sample_from_file(
                Some(mixer.clone()),
                &format!(
                    "{}/normal-hitnormal.wav",
                    asset_loader.borrow().skin.base_path
                ),
                1,
                0,
            )
            .unwrap();

        AudioManager {
            bass,
            asset_loader,
            mixer,
            main_track,
            main_track_clock,
            test_sample,
        }
    }

    pub fn play_test_sample(&mut self, pan: f32) {
        /*
        let channel = self
            .test_sample
            .get_channel(
                bass_sys::BASS_SAMCHAN_STREAM | bass_sys::BASS_STREAM_DECODE,
                true,
            )
            .unwrap();
        channel.set_attrib(bass_sys::BASS_ATTRIB_PAN, pan);
        channel.set_attrib(bass_sys::BASS_ATTRIB_VOL, 0.8);
        self.mixer
            .add_channel(
                channel,
                bassmix_sys::BASS_MIXER_CHAN_NORAMPIN | bass_sys::BASS_STREAM_AUTOFREE,
            )
            .unwrap();
        */
        self.test_sample.play_mixer(pan, 0.8);
    }

    pub fn seek_music(&mut self, pos: f64) -> bool {
        self.main_track_clock.seek(pos)
    }

    pub fn resume_music(&mut self) {
        self.main_track_clock.start();
    }

    pub fn pause_music(&mut self) {
        self.main_track_clock.pause();
    }

    pub fn music_pos(&self) -> f64 {
        self.main_track_clock.get_time()
    }

    pub fn update(&mut self) {
        self.main_track_clock.update();
    }
}
