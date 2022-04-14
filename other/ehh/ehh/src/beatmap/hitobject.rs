use log::warn;

use crate::math::{self, Line};

use super::{
    timing_point::{self, TimingPoint},
    Difficulty,
};

#[derive(PartialEq)]
pub enum HitObjectType {
    Circle,
    Slider,
    Spinner,
    //Hold, // no idea how this should be handled
}
impl Default for HitObjectType {
    fn default() -> Self {
        HitObjectType::Circle
    }
}

#[derive(Default)]
pub struct SliderTick {
    pub time: i32,
    pub pos: math::Vector2,
    pub fade_time: (i32, i32),
    pub angle: f32,
    pub is_repeat: bool,
}

impl SliderTick {
    pub fn new(
        time: i32,
        pos: math::Vector2,
        fade_time: (i32, i32),
        angle: f32,
        is_repeat: bool,
    ) -> SliderTick {
        SliderTick {
            time,
            pos,
            fade_time,
            angle,
            is_repeat,
        }
    }
}

#[derive(Default)]
pub struct SliderInfo {
    pub spatial_length: f64,
    pub slides: i32, // yes, this can be negative...
    pub curve: crate::curve::Curve,
    pub ball_path: Vec<(i32, i32, Line)>,

    pub velocity: f64,

    pub score_times: Vec<i32>,
    pub small_ticks: Vec<SliderTick>,
    pub end_ticks: Vec<SliderTick>,
}

#[derive(Default)]
pub struct HitObject {
    // TODO: do i really need unstacked position anywhere
    pub start_pos: math::Vector2,
    pub unstacked_start_pos: math::Vector2,
    pub end_pos: math::Vector2,
    pub unstacked_end_pos: math::Vector2,
    pub start: i32,
    pub end: i32,
    pub object_type: HitObjectType,
    pub stack_offset: math::Vector2,
    pub stack_count: i32,
    pub time_preempt: i32,
    pub flags: i32,

    pub slider_info: Option<Box<SliderInfo>>,
}

impl Ord for HitObject {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.start == other.start {
            other.is_new_combo().cmp(&self.is_new_combo())
        } else {
            self.start.cmp(&other.start)
        }
    }
}

impl PartialOrd for HitObject {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HitObject {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.is_new_combo() == other.is_new_combo()
    }
}
impl Eq for HitObject {}

impl HitObject {
    pub fn is_new_combo(&self) -> bool {
        self.flags & 4 != 0
    }

    pub fn time_at_length(&self, length: f32) -> i32 {
        if let Some(slider_info) = &self.slider_info {
            self.start + ((length / slider_info.velocity as f32) * 1000.0) as i32
        } else {
            self.start
        }
    }

    pub fn pos_at_time(&self, time: i32) -> math::Vector2 {
        // WARNING: if the slider is at exactly the end of either side, it will teleport to the other end!!!!
        // this is because 1.0 % 1.0 == 0.0
        if let Some(slider_info) = &self.slider_info {
            let time = time.clamp(self.start, (self.end).max(self.start));
            let mut pos = (time - self.start) as f32
                / ((self.end - self.start) as f32 / slider_info.slides as f32);

            pos = if pos % 2.0 > 1.0 {
                1.0 - pos % 1.0
            } else {
                pos % 1.0
            };

            if slider_info.curve.length > 0.0 && !slider_info.curve.lines.is_empty() {
                slider_info.curve.point_at(pos) - self.stack_offset
            } else {
                self.start_pos
            }
        } else {
            self.start_pos
        }
    }

    pub fn ball_pos_at_time(&self, time: i32) -> (math::Vector2, f32) {
        if let Some(slider_info) = &self.slider_info {
            let time = time.clamp(self.start, (self.end).max(self.start));

            if !slider_info.ball_path.is_empty() {
                let target_idx = slider_info
                    .ball_path
                    .partition_point(|&x| x.1 < time)
                    .min(slider_info.ball_path.len() - 1);
                let line = &slider_info.ball_path[target_idx];
                let pos = line
                    .2
                    .point_at((time - line.0) as f32 / (line.1 - line.0) as f32);
                let ang = (line.2.p2.y - line.2.p1.y).atan2(line.2.p2.x - line.2.p1.x);
                (pos - self.stack_offset, ang)
            } else {
                (self.start_pos, 0.0)
            }
        } else {
            (self.start_pos, 0.0)
        }
    }

    pub fn angle_at_time(&self, time: i32) -> f32 {
        // TODO: duplicated code...
        if let Some(slider_info) = &self.slider_info {
            let time = time.clamp(self.start, self.end);
            let mut pos = (time - self.start) as f32
                / ((self.end - self.start) as f32 / slider_info.slides as f32);

            let rev = pos % 2.0 > 1.0;
            pos = if rev { 1.0 - pos % 1.0 } else { pos % 1.0 };

            if slider_info.curve.length > 0.0 {
                slider_info.curve.angle_at(pos) - if rev { std::f32::consts::PI } else { 0.0 }
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    // for getting end position, tick timing points, etc
    pub fn recalculate_slider(
        &mut self,
        beatmap_version: i32,
        timing_points: &[TimingPoint],
        difficulty: &Difficulty,
    ) {
        if let Some(slider_info) = &mut self.slider_info {
            let beat_length = timing_point::beat_length_at(timing_points, self.start as f64, true);
            slider_info.velocity = if beat_length > 0.0 {
                difficulty.slider_scoring_point_distance
                    * difficulty.slider_tick_rate
                    * (1000.0 / beat_length)
            } else {
                difficulty.slider_scoring_point_distance * difficulty.slider_tick_rate
            };

            let tick_distance = if beatmap_version < 8 {
                difficulty.slider_scoring_point_distance
            } else {
                difficulty.slider_scoring_point_distance
                    / timing_point::bpm_multiplier_at(timing_points, self.start as f64) as f64
            }
            .minimum(slider_info.spatial_length);

            let mut current_time = self.start as f64;
            let mut scoring_distance = 0.0;
            let mut scoring_length_total = 0.0;

            let mut sum_len: f64 = 0.0;
            for x in &slider_info.curve.lines {
                sum_len += x.length() as f64;
            }

            let mut outer_p1 = self.unstacked_start_pos;
            let mut outer_p2 = self.unstacked_start_pos;

            // avoid trying to create more than a million ticks
            const TICK_COUNT_THRESHOLD: f64 = 1_000_000.0;
            let estimated_tick_count = slider_info.spatial_length / tick_distance;

            if estimated_tick_count > TICK_COUNT_THRESHOLD {
                warn!(
                    "Skipping ticks for a slider at {} because it tried to create {} ticks",
                    self.start, estimated_tick_count
                );
            }

            slider_info
                .ball_path
                .reserve(slider_info.curve.lines.len() * slider_info.slides as usize);

            for i in 0..slider_info.slides {
                let mut distance_to_end = sum_len;
                let mut skip_tick = !tick_distance.is_finite()
                    || tick_distance <= 0.001
                    || estimated_tick_count > TICK_COUNT_THRESHOLD;
                let reverse = (i % 2) == 1;
                let reverse_start_time = current_time as i32;

                let iter = if reverse {
                    itertools::Either::Left((0..slider_info.curve.lines.len()).rev())
                } else {
                    itertools::Either::Right(0..slider_info.curve.lines.len())
                };

                for j in iter {
                    let l = &slider_info.curve.lines[j];
                    let distance = l.length() as f64;

                    let (p1, p2) = if reverse { (l.p2, l.p1) } else { (l.p1, l.p2) };
                    outer_p1 = p1;
                    outer_p2 = p2;

                    let duration = 1000.0 * distance / slider_info.velocity;

                    // can't just track the ball with pos_at_time because of the modulo shit :/
                    slider_info.ball_path.push((
                        current_time as i32,
                        (current_time + duration) as i32,
                        Line::new(p1, p2),
                    ));

                    current_time += duration;
                    scoring_distance += distance;

                    while scoring_distance >= tick_distance && !skip_tick {
                        scoring_length_total += tick_distance;
                        scoring_distance -= tick_distance;
                        distance_to_end -= tick_distance;

                        skip_tick = distance_to_end <= 0.01 * slider_info.velocity;
                        if skip_tick {
                            break;
                        }

                        // can't use time_at_length here because borrow checker lol!!!!
                        let score_time = self.start
                            + ((scoring_length_total as f32 / slider_info.velocity as f32) * 1000.0)
                                as i32;
                        slider_info.score_times.push(score_time);

                        let point_ratio = 1.0 - (scoring_distance as f32 / p1.distance(p2));
                        let tick_pos = p1 + (p2 - p1) * point_ratio;

                        // TODO: handle the scaling animation and stuff
                        let fade_time = if i == 0 {
                            let start_time = (score_time - self.start) / 2 + self.start
                                - difficulty.preempt_slider_complete;
                            (start_time, start_time + 150)
                        } else {
                            let display_start_time =
                                reverse_start_time + (score_time - reverse_start_time) / 2;
                            (display_start_time - 200, display_start_time)
                        };

                        slider_info
                            .small_ticks
                            .push(SliderTick::new(score_time, tick_pos, fade_time, 0.0, false));
                    }
                }

                scoring_length_total += scoring_distance;
                let score_time = self.start
                    + ((scoring_length_total as f32 / slider_info.velocity as f32) * 1000.0) as i32;
                slider_info.score_times.push(score_time);

                if skip_tick {
                    scoring_distance = 0.0;
                } else {
                    scoring_length_total -= tick_distance - scoring_distance;
                    scoring_distance = tick_distance - scoring_distance;
                }

                let appear_time = if i == 0 {
                    self.start - difficulty.preempt
                } else {
                    reverse_start_time - (current_time as i32 - reverse_start_time)
                };
                slider_info.end_ticks.push(SliderTick::new(
                    current_time as i32,
                    outer_p2,
                    (appear_time, appear_time + 150),
                    (outer_p1.y - outer_p2.y).atan2(outer_p1.x - outer_p2.x),
                    i != slider_info.slides - 1,
                ));
            }

            self.unstacked_end_pos = outer_p2;
            self.end = current_time as i32;

            if current_time.is_nan() {
                // it's possible to make the beat length so low that it basically acts like 0 for most operations
                // it then causes a division by zero during the tick calculation code, causing the end time to be NaN
                // C# converts NaN to int32.MinValue, Rust converts it to 0
                // it lets all kinds of fun stuff happen like letting the map end earlier than it should
                // i'm not even going to try emulating it for now
                // see sky_delta - Grenade (MinG3012) [Negative Infinity Fragmentation]
                warn!(
                    "Slider at {} has a NaN end time and probably won't load properly!",
                    self.start
                );
                self.end = self.start;
            }

            if !slider_info.score_times.is_empty() {
                let len = slider_info.score_times.len();
                slider_info.score_times[len - 1] =
                    (self.start + (self.end - self.start) / 2).max(self.end - 36);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use crate::Beatmap;

    fn test_slider_ticks_inner(path: &str, expected: &[i32]) {
        let beatmap = Beatmap::parse("", &mut BufReader::new(File::open(path).unwrap())).unwrap();
        let obj = &beatmap.hit_objects[0];
        let slider_info = obj.slider_info.as_ref().unwrap();
        println!("expected: {:?}", expected);
        println!("output:   {:?}", slider_info.score_times);
        assert_eq!(slider_info.score_times.len(), expected.len());
        for i in 0..expected.len() {
            assert_eq!(slider_info.score_times[i], expected[i]);
        }
    }

    #[test]
    fn test_slider_ticks() {
        //test_slider_ticks_inner("test/simple_slider.osu", 14);
        test_slider_ticks_inner(
            "test/simple_slider_with_repeats.osu",
            &[
                1083, 1166, 1250, 1333, 1416, 1500, 1583, 1666, 1750, 1833, 1916, 2000, 2083, 2111,
                2138, 2222, 2305, 2388, 2472, 2555, 2638, 2722, 2805, 2888, 2972, 3055, 3138, 3186,
            ],
        );
        test_slider_ticks_inner("test/nan_slider_velocity.osu", &[1464]);
    }
}
