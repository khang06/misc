#![allow(dead_code)]
#![feature(float_minimum_maximum)]
#![feature(thread_is_running)]

pub mod app;
pub mod framework;

pub mod beatmap;
pub mod curve;
pub mod math;
pub mod num_util;

pub use beatmap::Beatmap;
pub use beatmap::BeatmapParseErr;
