pub struct Difficulty {
    pub approach_rate: f32,
    pub circle_size: f32,
    pub hp_drain: f32,
    pub overall_difficulty: f32,

    pub hit_50: i32,
    pub hit_100: i32,
    pub hit_300: i32,
    pub preempt: i32,
    pub preempt_slider_complete: i32,

    pub obj_radius: f32,
    pub stack_offset: f32,

    pub slider_multiplier: f64,
    pub slider_tick_rate: f64,

    pub slider_scoring_point_distance: f64,
}

fn map_diff_range(diff: f32, min: f32, mid: f32, max: f32) -> f32 {
    // TODO: adjust for hr/ez
    match diff {
        diff if diff > 5.0 => mid + (max - mid) * (diff - 5.0) / 5.0,
        diff if diff < 5.0 => mid - (mid - min) * (5.0 - diff) / 5.0,
        _ => mid,
    }
}

impl Difficulty {
    pub fn recalculate(&mut self) {
        self.hit_50 = map_diff_range(self.overall_difficulty, 200.0, 150.0, 100.0) as i32;
        self.hit_100 = map_diff_range(self.overall_difficulty, 140.0, 100.0, 60.0) as i32;
        self.hit_300 = map_diff_range(self.overall_difficulty, 80.0, 50.0, 20.0) as i32;

        self.preempt = map_diff_range(self.approach_rate, 1800.0, 1200.0, 450.0) as i32;
        self.preempt_slider_complete = (self.preempt as f32 * (2.0 / 3.0)) as i32;

        // stolen from https://github.com/McKay42/McOsu/blob/f6c96abe53b8c2b124366591c672326925e003d4/src/App/Osu/OsuGameRules.h#L344
        self.obj_radius =
            ((1.0 - 0.7 * (self.circle_size - 5.0) / 5.0) / 2.0) * 128.0 * 1.00041 / 2.0;
        self.stack_offset = self.obj_radius / 10.0;

        self.slider_scoring_point_distance =
            (100.0 * self.slider_multiplier) / self.slider_tick_rate;
    }

    pub fn new(
        approach_rate: f32,
        circle_size: f32,
        hp_drain: f32,
        overall_difficulty: f32,
        slider_multiplier: f64,
        slider_tick_rate: f64,
    ) -> Difficulty {
        // TODO: adjust for hr/ez
        let mut out = Difficulty {
            approach_rate,
            circle_size,
            hp_drain,
            overall_difficulty,

            hit_50: 0,
            hit_100: 0,
            hit_300: 0,
            preempt: 0,
            preempt_slider_complete: 0,

            obj_radius: 0.0,
            stack_offset: 0.0,

            slider_multiplier,
            slider_tick_rate,

            slider_scoring_point_distance: 0.0,
        };
        out.recalculate();
        out
    }
}

impl Default for Difficulty {
    fn default() -> Difficulty {
        Difficulty::new(5.0, 5.0, 5.0, 5.0, 1.4, 1.0)
    }
}
