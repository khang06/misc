use std::{cell::RefCell, rc::Rc};

use intervaltree::IntervalTree;

use crate::{
    beatmap::{HitObject, HitObjectType},
    framework::render::{DrawBatch, Origin},
    math::{interp_time, Easing, Vector2},
    Beatmap,
};

use super::{
    asset_loader::AssetLoader,
    audio_manager::AudioManager,
    game::{OSU_PLAYFIELD_HEIGHT, OSU_PLAYFIELD_WIDTH},
};

// only what's needed for standard
bitflags::bitflags! {
    pub struct IncreaseScoreType: i32 {
        const MISS = -131072;
        const IGNORE = 0;
        const KATU_ADDITION = 2;
        const GEKI_ADDITION = 4;
        const SLIDER_TICK = 8;
        const SLIDER_REPEAT = 64;
        const SLIDER_END = 128;
        const HIT_50 = 256;
        const HIT_100 = 512;
        const HIT_300 = 1024;
        const HIT_100K = Self::HIT_100.bits | Self::KATU_ADDITION.bits;
        const HIT_300K = Self::HIT_300.bits | Self::KATU_ADDITION.bits;
        const HIT_300G = Self::HIT_300.bits | Self::GEKI_ADDITION.bits;
        const SPINNER_SPIN = 4096;
        const SPINNER_SPIN_POINTS = 8192;
        const SPINNER_BONUS = 16384;
    }
}

pub struct GameplaySliderInfo {
    pub is_sliding: bool,
    pub slide_update: i32,
}

pub struct GameplayHitObject {
    audio_manager: Rc<RefCell<AudioManager>>,
    beatmap: Rc<Beatmap>,
    inner_obj_idx: usize,
    combo_color: u32,
    pub hit_time: Option<i32>,
    pub slider_info: Option<GameplaySliderInfo>,
}

impl GameplayHitObject {
    pub fn new(
        audio_manager: Rc<RefCell<AudioManager>>,
        beatmap: Rc<Beatmap>,
        inner_obj_idx: usize,
    ) -> GameplayHitObject {
        let slider_info = if beatmap.hit_objects[inner_obj_idx].object_type == HitObjectType::Slider
        {
            Some(GameplaySliderInfo {
                is_sliding: false,
                slide_update: beatmap.hit_objects[inner_obj_idx].start - 1000, // random number
            })
        } else {
            None
        };

        GameplayHitObject {
            audio_manager,
            beatmap,
            combo_color: 0x0000FF, // BGR, just a placeholder for now...
            inner_obj_idx,
            hit_time: None,
            slider_info,
        }
    }

    pub fn inner_obj(&self) -> &HitObject {
        &self.beatmap.hit_objects[self.inner_obj_idx]
    }

    pub fn start_pos(&self) -> Vector2 {
        self.inner_obj().start_pos
    }

    pub fn end_pos(&self) -> Vector2 {
        self.inner_obj().end_pos
    }

    pub fn start_time(&self) -> i32 {
        self.inner_obj().start
    }

    pub fn end_time(&self) -> i32 {
        self.inner_obj().end
    }

    pub fn is_hitcircle(&self) -> bool {
        self.inner_obj().object_type == HitObjectType::Circle
    }

    pub fn is_slider(&self) -> bool {
        self.inner_obj().object_type == HitObjectType::Slider
    }

    pub fn is_spinner(&self) -> bool {
        self.inner_obj().object_type == HitObjectType::Spinner
    }

    pub fn start_slide(&mut self, time: i32) {
        if let Some(slider_info) = self.slider_info.as_mut() {
            if !slider_info.is_sliding {
                slider_info.is_sliding = true;
                slider_info.slide_update = time;
            }
        }
    }

    pub fn stop_slide(&mut self, time: i32) {
        if let Some(slider_info) = self.slider_info.as_mut() {
            if slider_info.is_sliding {
                slider_info.is_sliding = false;
                slider_info.slide_update = time;
            }
        }
    }

    pub fn play_hitsound(&mut self) {
        // TODO: placeholder. needs to properly get all the data required like timing points and shit
        let pan = (self.start_pos().x / 512.0 - 0.5) * 0.8;
        self.audio_manager.borrow_mut().play_test_sample(pan);
    }

    pub fn hit(&mut self, hit_time: i32) -> IncreaseScoreType {
        self.hit_time = Some(hit_time);

        let accuracy = (hit_time - self.start_time()).abs();
        let diff = &self.beatmap.difficulty;
        let hit_value = match accuracy {
            x if x < diff.hit_300 => IncreaseScoreType::HIT_300,
            x if x < diff.hit_100 => IncreaseScoreType::HIT_100,
            x if x < diff.hit_50 => IncreaseScoreType::HIT_50,
            _ => IncreaseScoreType::MISS,
        };

        self.play_hitsound();

        if self.is_slider() {
            self.start_slide(hit_time);
        }

        hit_value
    }
}

pub struct HitObjectManager {
    asset_loader: Rc<RefCell<AssetLoader>>,
    audio_manager: Rc<RefCell<AudioManager>>,
    pub beatmap: Rc<Beatmap>,
    gameplay_objs: IntervalTree<i32, Rc<RefCell<GameplayHitObject>>>,
    visible_objs: Vec<Rc<RefCell<GameplayHitObject>>>,

    batch: DrawBatch,
}

impl HitObjectManager {
    pub fn new(
        width: f32,
        height: f32,
        asset_loader: Rc<RefCell<AssetLoader>>,
        audio_manager: Rc<RefCell<AudioManager>>,
        beatmap: Rc<Beatmap>,
    ) -> HitObjectManager {
        // 384 * (height / 480) = height * 0.8
        let scale = OSU_PLAYFIELD_HEIGHT as f32 / (height * 0.8);
        let extra_x = (width * scale - OSU_PLAYFIELD_WIDTH as f32) / 2.0;
        let extra_y =
            (height * scale - OSU_PLAYFIELD_WIDTH as f32) / 4.0 * 3.0 - 16.0 * (height / 480.0);
        let ortho = cgmath::ortho(
            -extra_x,
            width * scale - extra_x,
            height * scale + extra_y,
            extra_y,
            -1.0f32,
            1.0f32,
        );

        // TODO: i can totally just do this in one iteration, but it would probably really suck to read...
        let mut gameplay_objs = Vec::with_capacity(beatmap.hit_objects.len());
        for (i, _x) in beatmap.hit_objects.iter().enumerate() {
            gameplay_objs.push(Rc::new(RefCell::new(GameplayHitObject::new(
                audio_manager.clone(),
                beatmap.clone(),
                i,
            ))));
        }
        let gameplay_objs = IntervalTree::from_iter(gameplay_objs.into_iter().map(|x| {
            let start = x.borrow().start_time();
            let end = x.borrow().end_time() + 1;
            (start..end.max(start + 1), x)
        }));

        HitObjectManager {
            asset_loader,
            audio_manager,
            beatmap,
            gameplay_objs,
            visible_objs: Default::default(),
            batch: DrawBatch::new(ortho),
        }
    }

    pub fn visible_objs_count(&self) -> usize {
        self.visible_objs.len()
    }

    fn update_visible_objs(&mut self, time: i32) {
        let preempt = self.beatmap.difficulty.preempt;

        self.visible_objs.clear();
        for x in self
            .gameplay_objs
            .query((time - preempt)..(time + preempt))
            .map(|x| &x.value)
        {
            self.visible_objs.push(x.clone());
        }
    }

    // just for debug purposes...
    fn update_force_hit(&mut self, time: i32) {
        for x in &self.visible_objs {
            let mut x = x.borrow_mut();
            let start = x.start_time();
            if start <= time && x.hit_time.is_none() {
                x.hit(start);
            }
        }
    }

    pub fn update(&mut self, time: i32) {
        self.update_visible_objs(time);
        self.update_force_hit(time);
    }

    fn draw_hitcircles(&mut self, time: i32) {
        let hitcircle = self.asset_loader.borrow_mut().lookup_tex("hitcircle");
        let hitcircle_scale = self.beatmap.difficulty.obj_radius * 2.0
            / hitcircle.width.max(hitcircle.height)
            * hitcircle.dpi_scale;

        let hitcircleoverlay = self
            .asset_loader
            .borrow_mut()
            .lookup_tex("hitcircleoverlay");
        let hitcircleoverlay_scale = self.beatmap.difficulty.obj_radius * 2.0
            / hitcircleoverlay.width.max(hitcircleoverlay.height)
            * hitcircleoverlay.dpi_scale;

        let approachcircle = self.asset_loader.borrow_mut().lookup_tex("approachcircle");
        let approachcircle_scale = self.beatmap.difficulty.obj_radius * 2.0
            / approachcircle.width.max(approachcircle.height)
            * approachcircle.dpi_scale;

        let preempt = self.beatmap.difficulty.preempt;
        let hit_50 = self.beatmap.difficulty.hit_50;

        for x in &self.visible_objs {
            let x = x.borrow();
            let start_time = x.start_time();
            let hit_time = x.hit_time;

            // TODO: this sucks ass!!!!!
            let circle_alpha = if time < start_time {
                (interp_time(
                    0.0,
                    1.0,
                    (start_time - preempt) as f32,
                    (start_time - preempt + 400) as f32,
                    time as f32,
                    Easing::Linear,
                ) * 255.0)
                    .clamp(0.0, 255.0) as u32
            } else if let Some(hit_time) = hit_time {
                (interp_time(
                    1.0,
                    0.0,
                    hit_time as f32,
                    (hit_time + 240) as f32,
                    time as f32,
                    Easing::Linear,
                ) * 255.0)
                    .clamp(0.0, 255.0) as u32
            } else if time < start_time + hit_50 {
                0xFF
            } else {
                0x0
            };
            let approach_alpha = (interp_time(
                0.0,
                0.9,
                (start_time - preempt) as f32,
                (start_time - preempt + 400 * 2).min(start_time) as f32,
                time as f32,
                Easing::Linear,
            ) * 255.0)
                .clamp(0.0, 255.0) as u32;

            // TODO: base this on arm time after getting input and shit done!!!
            // also remember to do circle_alpha too
            let circle_scale = if let Some(hit_time) = hit_time {
                interp_time(
                    1.0,
                    1.4,
                    hit_time as f32,
                    (hit_time + 240) as f32,
                    time as f32,
                    Easing::Linear,
                )
            } else {
                1.0
            };

            self.batch.add(
                hitcircle.clone(),
                x.start_pos(),
                hitcircle_scale * circle_scale,
                Origin::Center,
                x.combo_color | (circle_alpha << 24),
                0.0,
            );

            self.batch.add(
                hitcircleoverlay.clone(),
                x.start_pos(),
                hitcircleoverlay_scale * circle_scale,
                Origin::Center,
                0xFFFFFF | (circle_alpha << 24),
                0.0,
            );

            // approach circle
            if time <= start_time && x.hit_time.is_none() {
                let scale = interp_time(
                    4.0,
                    1.0,
                    (start_time - preempt) as f32,
                    start_time as f32,
                    time as f32,
                    Easing::Linear,
                );
                self.batch.add(
                    approachcircle.clone(),
                    x.start_pos(),
                    approachcircle_scale * scale,
                    Origin::Center,
                    x.combo_color | (approach_alpha << 24),
                    0.0,
                );
            }
        }
    }

    fn draw_slider_objs(&mut self, time: i32) {
        // slider stuff (tracking circle, ticks, etc) are drawn under all hitcircles
        let tick_tex = self
            .asset_loader
            .borrow_mut()
            .lookup_tex("sliderscorepoint");
        let repeat_tex = self.asset_loader.borrow_mut().lookup_tex("reversearrow");
        let ball_tex = self.asset_loader.borrow_mut().lookup_anim("sliderb", false);
        let tex_scale = self.beatmap.difficulty.obj_radius * 2.0 / 128.0;

        for x in &self.visible_objs {
            let x = x.borrow_mut();
            if let Some(slider_info) = &x.inner_obj().slider_info {
                // TODO: make these rely on the slider state instead!!!
                if time >= x.start_time() && time <= x.end_time() {
                    let (pos, ang) = x.inner_obj().ball_pos_at_time(time);
                    self.batch.add(
                        ball_tex.get_tex(x.start_time(), time),
                        pos,
                        tex_scale,
                        Origin::Center,
                        0xFFFFFFFF,
                        ang,
                    );
                }

                for tick in &slider_info.small_ticks {
                    if tick.time > time {
                        let alpha = (interp_time(
                            0.0,
                            1.0,
                            tick.fade_time.0 as f32,
                            tick.fade_time.1 as f32,
                            time as f32,
                            Easing::Linear,
                        ) * 255.0)
                            .clamp(0.0, 255.0) as u32;

                        self.batch.add(
                            tick_tex.clone(),
                            tick.pos,
                            tex_scale, // TODO: no idea if this is right!!!!
                            Origin::Center,
                            0xFFFFFF | (alpha << 24),
                            0.0,
                        );
                    }
                }

                for tick in &slider_info.end_ticks {
                    if tick.is_repeat {
                        let alpha = if time < tick.time {
                            (interp_time(
                                0.0,
                                1.0,
                                tick.fade_time.0 as f32,
                                tick.fade_time.1 as f32,
                                time as f32,
                                Easing::Linear,
                            ) * 255.0)
                                .clamp(0.0, 255.0) as u32
                        } else {
                            (interp_time(
                                1.0,
                                0.0,
                                tick.time as f32,
                                (tick.time + 240) as f32,
                                time as f32,
                                Easing::Linear,
                            ) * 255.0)
                                .clamp(0.0, 255.0) as u32
                        };
                        let scale = if time >= tick.time {
                            interp_time(
                                1.0,
                                1.4,
                                tick.time as f32,
                                (tick.time + 240) as f32,
                                time as f32,
                                Easing::Linear,
                            )
                        } else if time >= tick.fade_time.0 {
                            let elapsed = time - tick.fade_time.0;
                            let phase_start = tick.fade_time.0 + elapsed / 300 * 300;
                            let length = 300.min(tick.time - phase_start);
                            interp_time(
                                1.3,
                                1.0,
                                phase_start as f32,
                                (phase_start + length) as f32,
                                time as f32,
                                Easing::OutQuad,
                            )
                        } else {
                            1.0
                        };

                        self.batch.add(
                            repeat_tex.clone(),
                            tick.pos,
                            tex_scale * scale,
                            Origin::Center,
                            0xFFFFFF | (alpha << 24),
                            tick.angle,
                        );
                    }
                }
            }
        }
    }

    pub fn draw(&mut self, time: i32) {
        self.draw_slider_objs(time);
        self.draw_hitcircles(time);
        self.batch.draw();
    }
}
