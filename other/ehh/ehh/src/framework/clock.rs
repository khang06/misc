use std::{rc::Rc, time::Instant};

use super::bass::{BassChannelCommon, BassMixer, BassStream};

pub trait Clock {
    fn update(&mut self);
    fn start(&mut self);
    fn pause(&mut self);
    fn seek(&mut self, pos: f64) -> bool; // seeking can fail if the time range is unsupported

    fn is_running(&self) -> bool;
    fn get_time(&self) -> f64;
    fn get_rate(&self) -> f64;
    fn get_elapsed_frame_time(&self) -> f64;
}

// TODO: rate accumulation once i get around to that...
pub struct InstantClock {
    instant: Instant,
    time: f64, // user-facing, updated per-frame
    start: f64,
    elapsed: f64,
    seek_offset: f64,
    running: bool,
    last_frame_time: f64,
}

impl InstantClock {
    pub fn new(start: bool) -> InstantClock {
        let mut ret = InstantClock {
            instant: Instant::now(),
            time: 0.0,
            start: 0.0,
            elapsed: 0.0,
            seek_offset: 0.0,
            running: false,
            last_frame_time: 0.0,
        };
        if start {
            ret.start();
        }
        ret.update();
        ret
    }
}

impl Clock for InstantClock {
    fn update(&mut self) {
        self.last_frame_time = self.time;

        let mut elapsed = self.elapsed;
        if self.running {
            elapsed += self.instant.elapsed().as_secs_f64() * 1000.0 - self.start;
        }
        self.time = elapsed;
    }

    fn start(&mut self) {
        if !self.running {
            self.start = self.instant.elapsed().as_secs_f64() * 1000.0;
            self.running = true;
        }
    }

    fn pause(&mut self) {
        if self.running {
            self.elapsed += self.instant.elapsed().as_secs_f64() * 1000.0 - self.start;
            self.running = false;
        }
    }

    fn seek(&mut self, offset: f64) -> bool {
        self.seek_offset = offset - self.time;
        true
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn get_time(&self) -> f64 {
        self.time + self.seek_offset
    }

    fn get_rate(&self) -> f64 {
        1.0
    }

    fn get_elapsed_frame_time(&self) -> f64 {
        self.time - self.last_frame_time
    }
}

pub struct OffsetClock {
    inner: Box<dyn Clock>,
    offset: f64,
}

impl OffsetClock {
    pub fn new(inner: Box<dyn Clock>, offset: f64) -> OffsetClock {
        OffsetClock { inner, offset }
    }

    pub fn set_offset(&mut self, offset: f64) {
        self.offset = offset;
    }
}

impl Clock for OffsetClock {
    fn update(&mut self) {
        self.inner.update();
    }

    fn start(&mut self) {
        self.inner.start();
    }

    fn pause(&mut self) {
        self.inner.pause();
    }

    fn seek(&mut self, pos: f64) -> bool {
        self.inner.seek(pos + self.offset)
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    fn get_time(&self) -> f64 {
        self.inner.get_time() - self.offset
    }

    fn get_rate(&self) -> f64 {
        self.inner.get_rate()
    }

    fn get_elapsed_frame_time(&self) -> f64 {
        self.inner.get_elapsed_frame_time()
    }
}

// doesn't seem to require any interpolation?
pub struct BassStreamClock {
    mixer: Rc<BassMixer>,
    stream: Rc<BassStream>,
    time: f64,
    last_frame_time: f64,
}

impl BassStreamClock {
    pub fn new(mixer: Rc<BassMixer>, stream: Rc<BassStream>) -> BassStreamClock {
        BassStreamClock {
            mixer,
            stream,
            time: 0.0,
            last_frame_time: 0.0,
        }
    }
}

impl Clock for BassStreamClock {
    fn update(&mut self) {
        self.last_frame_time = self.time;
        self.time = self.stream.get_mixer_position();
    }

    fn start(&mut self) {
        self.mixer.resume_channel(self.stream.clone());
    }

    fn pause(&mut self) {
        self.mixer.pause_channel(self.stream.clone());
    }

    fn seek(&mut self, pos: f64) -> bool {
        self.stream.set_mixer_position(pos)
    }

    fn is_running(&self) -> bool {
        self.stream.get_mixer_is_active()
    }

    fn get_time(&self) -> f64 {
        self.time
    }

    fn get_rate(&self) -> f64 {
        // TODO: need bassfx for rate changing
        1.0
    }

    fn get_elapsed_frame_time(&self) -> f64 {
        self.time - self.last_frame_time
    }
}

// osu-framework DecouplableInterpolatingFramedClock without the interpolating part
// also always runs in decoupled mode
pub struct DecoupledClock {
    source_clock: Box<dyn Clock>,
    decoupled_clock: InstantClock,
    current_time: f64,
    elapsed_frame_time: f64,
}

impl DecoupledClock {
    pub fn new(source_clock: Box<dyn Clock>) -> DecoupledClock {
        DecoupledClock {
            source_clock,
            decoupled_clock: InstantClock::new(false),
            current_time: 0.0,
            elapsed_frame_time: 0.0,
        }
    }

    fn use_source_time(&self) -> bool {
        self.is_running() && self.source_clock.is_running()
    }

    fn proposed_time(&self) -> f64 {
        if self.use_source_time() {
            self.source_clock.get_time()
        } else {
            self.decoupled_clock.get_time()
        }
    }

    fn proposed_frame_time(&self) -> f64 {
        if self.use_source_time() {
            self.source_clock.get_elapsed_frame_time()
        } else {
            self.decoupled_clock.get_elapsed_frame_time()
        }
    }
}

impl Clock for DecoupledClock {
    fn update(&mut self) {
        self.source_clock.update();
        self.decoupled_clock.update();
        let source_running = self.source_clock.is_running();

        let proposed_time = self.proposed_time();
        let proposed_frame_time = self.proposed_frame_time();

        // keep trying to start up the source clock until it can actually handle the current time range
        if self.is_running() && !source_running {
            self.start();
        } else if self.is_running() && source_running {
            let diff = self.decoupled_clock.get_time() - self.source_clock.get_time();
            if diff.abs() > 1.0 {
                //log::warn!("Resyncronizing clocks! Diff was {}ms", diff);
                self.decoupled_clock.seek(self.source_clock.get_time());
            }
        }

        self.elapsed_frame_time = proposed_frame_time;
        self.current_time = proposed_time;
    }

    fn start(&mut self) {
        if !self.source_clock.is_running() && self.source_clock.seek(self.proposed_time()) {
            self.source_clock.start();
        }
        self.decoupled_clock.start();
    }

    fn pause(&mut self) {
        self.decoupled_clock.pause();
        self.source_clock.pause();
    }

    fn seek(&mut self, pos: f64) -> bool {
        if !self.source_clock.seek(pos) {
            self.source_clock.pause();
        }
        self.decoupled_clock.seek(pos);
        self.update();
        true
    }

    fn is_running(&self) -> bool {
        self.decoupled_clock.is_running()
    }

    fn get_time(&self) -> f64 {
        self.current_time
    }

    fn get_rate(&self) -> f64 {
        1.0
    }

    fn get_elapsed_frame_time(&self) -> f64 {
        self.elapsed_frame_time
    }
}
