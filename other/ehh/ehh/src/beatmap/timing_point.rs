use super::*;

#[derive(Debug)]
pub enum SampleSet {
    All = -1,
    None,
    Normal,
    Soft,
    Drum,
}
impl BeatmapParse for SampleSet {
    fn parse(source: &str, line_num: u32) -> Result<Self, BeatmapParseErr> {
        match source.trim() {
            "All" => Ok(SampleSet::All),
            "None" => Ok(SampleSet::None),
            "Normal" => Ok(SampleSet::Normal),
            "Soft" => Ok(SampleSet::Soft),
            "Drum" => Ok(SampleSet::Drum),
            _ => Err(BeatmapParseErr::InvalidEnum(line_num)),
        }
    }
}
impl Default for SampleSet {
    fn default() -> Self {
        SampleSet::Normal
    }
}

#[derive(Default, Debug)]
pub struct TimingPoint {
    pub beat_length: f64,
    pub custom_sample_set: i32,
    pub kiai: bool,
    pub offset: f64,
    pub sample_set: SampleSet,
    pub time_signature: i32,
    pub timing_change: bool,
    pub volume: i32,
}

impl TimingPoint {
    pub fn bpm_multiplier(&self) -> f32 {
        if self.beat_length >= 0.0 {
            1.0
        } else {
            (-self.beat_length as f32).clamp(10.0, 1000.0) / 100.0
        }
    }
}

pub fn beat_length_at(timing_points: &[TimingPoint], time: f64, use_multiplier: bool) -> f64 {
    if timing_points.is_empty() {
        return 0.0;
    }

    let mut point = 0;
    let mut sample = 0;

    for (i, x) in timing_points.iter().enumerate() {
        if x.offset <= time {
            if timing_points[i].timing_change {
                point = i;
            } else {
                sample = i;
            }
        }
    }

    if use_multiplier && sample > point && timing_points[sample].beat_length < 0.0 {
        timing_points[point].beat_length * timing_points[sample].bpm_multiplier() as f64
    } else {
        timing_points[point].beat_length
    }
}

pub fn timing_point_at(timing_points: &[TimingPoint], time: f64) -> Option<&TimingPoint> {
    let mut ret = None;
    if !timing_points.is_empty() {
        for x in timing_points {
            if x.offset <= time {
                ret = Some(x);
            }
        }
        if ret.is_none() {
            ret = Some(&timing_points[0]);
        }
    }

    ret
}

pub fn bpm_multiplier_at(timing_points: &[TimingPoint], time: f64) -> f32 {
    let point = timing_point_at(timing_points, time);
    if let Some(point) = point {
        point.bpm_multiplier()
    } else {
        1.0
    }
}
