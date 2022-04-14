use std::io;
use std::io::BufRead;

use log::warn;

use crate::curve::*;
use crate::math::*;
use crate::num_util::*;

use super::hitobject::*;
use super::timing_point::*;
use super::*;

#[derive(Debug)]
pub enum BeatmapParseErr {
    IoError(io::Error),
    UnsupportedFormatVersion,
    UnsupportedMode,
    InvalidBool(u32), // only if you somehow get eof right before a bool
    InvalidInt(u32),
    InvalidFloat(u32),
    InvalidEnum(u32),
    InvalidTimingPoint(u32),
}
impl From<io::Error> for BeatmapParseErr {
    fn from(x: io::Error) -> BeatmapParseErr {
        BeatmapParseErr::IoError(x)
    }
}

pub trait BeatmapParse {
    fn parse(source: &str, line_num: u32) -> Result<Self, BeatmapParseErr>
    where
        Self: std::marker::Sized;
}

impl BeatmapParse for bool {
    fn parse(source: &str, line_num: u32) -> Result<Self, BeatmapParseErr> {
        if source.is_empty() {
            Err(BeatmapParseErr::InvalidBool(line_num))
        } else {
            Ok(source.starts_with('1'))
        }
    }
}
// TODO: figure out how to properly generalize these
impl BeatmapParse for i32 {
    fn parse(source: &str, line_num: u32) -> Result<Self, BeatmapParseErr> {
        match source.trim().parse::<Self>() {
            Ok(n) => Ok(n),
            Err(_) => Err(BeatmapParseErr::InvalidInt(line_num)),
        }
    }
}
impl BeatmapParse for f32 {
    fn parse(source: &str, line_num: u32) -> Result<Self, BeatmapParseErr> {
        match source.trim().parse::<Self>() {
            Ok(n) => Ok(n),
            Err(_) => Err(BeatmapParseErr::InvalidFloat(line_num)),
        }
    }
}
impl BeatmapParse for f64 {
    fn parse(source: &str, line_num: u32) -> Result<Self, BeatmapParseErr> {
        match source.trim().parse::<Self>() {
            Ok(n) => Ok(n),
            Err(_) => Err(BeatmapParseErr::InvalidFloat(line_num)),
        }
    }
}

enum Section {
    None,
    General,
    Metadata,
    Difficulty,
    TimingPoints,
    HitObjects,
}

impl Beatmap {
    const LATEST_FORMAT_VERSION: i32 = 14;

    pub fn parse(base_path: &str, file: &mut impl BufRead) -> Result<Self, BeatmapParseErr> {
        let mut beatmap = Beatmap {
            base_path: base_path.to_string(),
            format_version: Self::LATEST_FORMAT_VERSION,
            preview_time: -1,
            stack_leniency: 0.7,
            difficulty: Default::default(),

            ..Default::default()
        };

        // file parsing
        let mut buffer = String::new(); // avoid a ton of allocations

        // version should always be at the top
        // otherwise, osu just assumes it's the latest version
        match Self::next_line(file, &mut buffer) {
            Ok(_) => {
                if let Some(stripped) = buffer.strip_prefix("osu file format v") {
                    beatmap.format_version = i32::parse(stripped.trim(), 1)?
                }
            }
            Err(e) => return Err(e),
        }

        let mut section = Section::None;
        let mut line_num = 1u32;
        while let Ok(eof) = Self::next_line(file, &mut buffer) {
            line_num += 1;
            if eof {
                break;
            }

            // skip over blank lines and comments
            if buffer.trim().is_empty() {
                continue;
            }
            if buffer.starts_with("//") {
                continue;
            }

            // sections
            if buffer.starts_with('[') && buffer.trim_end().ends_with(']') {
                let section_name = &buffer[1..(buffer.trim_end().len() - 1)];
                section = match section_name {
                    "General" => Section::General,
                    "Metadata" => Section::Metadata,
                    "Difficulty" => Section::Difficulty,
                    "TimingPoints" => Section::TimingPoints,
                    "HitObjects" => Section::HitObjects,
                    _ => Section::None,
                };
                continue;
            }

            match section {
                Section::General => beatmap.handle_general(&buffer, line_num),
                Section::Metadata => beatmap.handle_metadata(&buffer, line_num),
                Section::Difficulty => beatmap.handle_difficulty(&buffer, line_num),
                Section::TimingPoints => {
                    beatmap.handle_timingpoints(&buffer, line_num);
                    Ok(())
                }
                Section::HitObjects => beatmap.handle_hitobjects(&buffer, line_num),
                _ => continue,
            }?
        }

        if beatmap.artist.is_empty() {
            beatmap.artist = beatmap.romanized_artist.clone();
        }
        if beatmap.title.is_empty() {
            beatmap.title = beatmap.romanized_title.clone();
        }

        // misc post-processing stuff
        // should be its own step, but for my purposes i don't need it as one
        beatmap.difficulty.recalculate();
        for x in &mut beatmap.hit_objects {
            if x.object_type == HitObjectType::Slider {
                x.recalculate_slider(
                    beatmap.format_version,
                    &beatmap.timing_points,
                    &beatmap.difficulty,
                );
            }
        }
        beatmap.process_stacking();

        Ok(beatmap)
    }

    fn handle_general(&mut self, line: &str, line_num: u32) -> Result<(), BeatmapParseErr> {
        if let Some((key, val)) = Self::split_key_val(line) {
            match key {
                "AlwaysShowPlayfield" => self.always_show_playfield = bool::parse(val, line_num)?,
                "AudioFilename" => self.audio_filename = val.to_owned(),
                "AudioHash" => self.audio_hash = val.to_owned(),
                "AudioLeadIn" => self.audio_lead_in = i32::parse(val, line_num)?,
                "Countdown" => {
                    self.countdown = match i32::parse(val, line_num)? {
                        0 => Countdown::None,
                        1 => Countdown::Normal,
                        2 => Countdown::HalfTime,
                        3 => Countdown::DoubleTime,
                        _ => return Err(BeatmapParseErr::InvalidEnum(line_num)),
                    }
                }
                "CountdownOffset" => self.countdown_offset = i32::parse(val, line_num)?,
                "CustomSamples" => self.custom_samples = bool::parse(val, line_num)?,
                "EpilespyWarning" => self.epilepsy_warning = bool::parse(val, line_num)?,
                "LetterboxInBreaks" => self.letterbox_in_breaks = bool::parse(val, line_num)?,
                "Mode" => {
                    self.mode = match i32::parse(val, line_num)? {
                        0 => Gamemode::Osu,
                        _ => return Err(BeatmapParseErr::UnsupportedMode),
                    }
                }
                "OverlayPosition" => self.overlay_position = OverlayPosition::parse(val, line_num)?,
                "PreviewTime" => self.preview_time = i32::parse(val, line_num)?,
                "SampleSet" => self.sample_set = SampleSet::parse(val, line_num)?,
                "SampleVolume" => self.sample_volume = i32::parse(val, line_num)?,
                "SamplesMatchPlaybackRate" => {
                    self.samples_match_playback_rate = bool::parse(val, line_num)?
                }
                "SkinPreference" => self.skin_preference = val.to_owned(),
                "SpecialStyle" => self.special_style = bool::parse(val, line_num)?,
                "StackLeniency" => self.stack_leniency = f32::parse(val, line_num)?,
                "TimelineZoom" => self.timeline_zoom = f32::parse(val, line_num)?,
                "WidescreenStoryboard" => self.widescreen_storyboard = bool::parse(val, line_num)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_metadata(&mut self, line: &str, line_num: u32) -> Result<(), BeatmapParseErr> {
        if let Some((key, val)) = Self::split_key_val(line) {
            match key {
                "Artist" => self.romanized_artist = val.to_owned(),
                "ArtistUnicode" => self.artist = val.to_owned(),
                "BeatmapID" => self.beatmap_id = i32::parse(val, line_num)?,
                "BeatmapSetID" => self.beatmap_set_id = i32::parse(val, line_num)?,
                "Creator" => self.creator = val.to_owned(),
                "Source" => self.source = val.to_owned(),
                "Tags" => self.tags = val.to_owned(),
                "Title" => self.romanized_title = val.to_owned(),
                "TitleUnicode" => self.title = val.to_owned(),
                "Version" => self.version = val.to_owned(),
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_difficulty(&mut self, line: &str, line_num: u32) -> Result<(), BeatmapParseErr> {
        if let Some((key, val)) = Self::split_key_val(line) {
            // NOTE: some of these are parsed as a byte if the beatmap version is less than 13
            // handling that here would only really make more maps error, which doesn't sound great
            match key {
                "ApproachRate" => {
                    self.difficulty.approach_rate = f32::parse(val, line_num)?.clamp(0.0, 10.0)
                }
                "CircleSize" => {
                    self.difficulty.circle_size = f32::parse(val, line_num)?.clamp(0.0, 10.0)
                }
                "HPDrainRate" => {
                    self.difficulty.hp_drain = f32::parse(val, line_num)?.clamp(0.0, 10.0)
                }
                "OverallDifficulty" => {
                    self.difficulty.overall_difficulty = f32::parse(val, line_num)?.clamp(0.0, 10.0)
                }
                "SliderMultiplier" => {
                    self.difficulty.slider_multiplier = f64::parse(val, line_num)?.clamp(0.4, 3.6)
                }
                "SliderTickRate" => {
                    self.difficulty.slider_tick_rate = f64::parse(val, line_num)?.clamp(0.5, 8.0)
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_timingpoints(&mut self, line: &str, line_num: u32) {
        // osu is very lenient on TimingPoints, simply skipping over it if there's an exception most of the time
        // i'll take skipping over not loading at all
        let mut new_point: TimingPoint = Default::default();
        let mut parse = || -> Result<(), BeatmapParseErr> {
            let mut split = line.split(',');
            let split_num = line.matches(',').count();

            new_point.time_signature = 4;
            new_point.timing_change = true;
            new_point.volume = 100;

            // TODO: this looks awful
            if split_num >= 1 {
                new_point.offset = f64::parse(split.next().unwrap().trim(), line_num)?;
                new_point.beat_length = f64::parse(split.next().unwrap().trim(), line_num)?;
                if new_point.beat_length >= 0.0 {
                    new_point.timing_change = true;
                }
                if self.format_version < 5 {
                    new_point.offset += 24.0;
                }
                if let Some(time_sig) = split.next() {
                    new_point.time_signature = i32::parse(time_sig.trim(), line_num)?;
                    if let Some(sample_set) = split.next() {
                        new_point.sample_set = match i32::parse(sample_set.trim(), line_num)? {
                            -1 => SampleSet::All,
                            0 => SampleSet::None,
                            1 => SampleSet::Normal,
                            2 => SampleSet::Soft,
                            3 => SampleSet::Drum,
                            //_ => return Err(BeatmapParseErr::InvalidEnum)
                            _ => SampleSet::None,
                        };
                        if let Some(custom_sample_set) = split.next() {
                            new_point.custom_sample_set =
                                i32::parse(custom_sample_set.trim(), line_num)?;
                            if let Some(volume) = split.next() {
                                new_point.volume = i32::parse(volume.trim(), line_num)?;
                                if let Some(timing_change) = split.next() {
                                    new_point.timing_change =
                                        bool::parse(timing_change.trim(), line_num)?;
                                    if let Some(kiai) = split.next() {
                                        new_point.kiai =
                                            i32::parse(kiai.trim(), line_num)? & 1 == 1;
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            } else {
                Err(BeatmapParseErr::InvalidTimingPoint(line_num))
            }
        };

        let res = parse();
        if res.is_ok() {
            self.timing_points.push(new_point);
        } else {
            warn!("Skipping timing point \"{}\" because parsing failed", line);
        }
    }

    #[allow(clippy::field_reassign_with_default)]
    fn handle_hitobjects(&mut self, line: &str, line_num: u32) -> Result<(), BeatmapParseErr> {
        let mut split = line.split(',');
        let split_num = line.matches(',').count() + 1;
        if split_num < 5 {
            return Ok(());
        }

        let mut new_obj: HitObject = Default::default();
        // TODO: flip for hr once mods are implemented
        let x = f32::parse(split.next().unwrap(), line_num)?.clamp(0.0, 512.0) as i32 as f32;
        let y = f32::parse(split.next().unwrap(), line_num)?.clamp(0.0, 512.0) as i32 as f32;
        new_obj.unstacked_start_pos = Vector2::new(x, y);
        new_obj.unstacked_end_pos = Vector2::new(x, y);
        new_obj.start = i32::parse(split.next().unwrap(), line_num)?;
        new_obj.end = new_obj.start;
        if self.format_version < 5 {
            new_obj.start += 24;
        }

        let type_flags = i32::parse(split.next().unwrap(), line_num)?;
        let _hitsound = split.next().unwrap();
        // TODO: this is disgusting
        new_obj.object_type = if type_flags & 1 > 0 {
            self.circle_count += 1;
            HitObjectType::Circle
        } else if type_flags & 2 > 0 {
            self.slider_count += 1;
            HitObjectType::Slider
        } else if type_flags & 8 > 0 {
            self.spinner_count += 1;
            HitObjectType::Spinner
        } else {
            // TODO: mania hold stuff? you can do cool stuff with that
            return Ok(());
        };
        new_obj.flags = type_flags;

        match new_obj.object_type {
            HitObjectType::Circle | HitObjectType::Spinner => {} // nothing to do here, sample stuff is ignored
            HitObjectType::Slider => {
                if split_num < 7 {
                    return Ok(());
                }
                let mut slider_info = SliderInfo::default();
                let slider_split = split.next().unwrap().split('|');

                let mut curve_type = CurveType::Catmull;
                let mut control_points: Vec<Vector2> = vec![new_obj.unstacked_start_pos];
                // the last slider type specified is the one that osu goes with
                for entry in slider_split {
                    if entry.len() == 1 {
                        curve_type = match entry {
                            "C" => CurveType::Catmull,
                            "B" => CurveType::Bezier,
                            "L" => CurveType::Linear,
                            "P" => CurveType::PerfectCircle,
                            _ => curve_type,
                        };
                    } else {
                        let mut point_split = entry.split(':');
                        let point_split_num = entry.matches(':').count() + 1;
                        if point_split_num < 2 {
                            return Ok(());
                        }
                        // i have no idea why osu parses it like this
                        // should be accurate to stable though...
                        let x =
                            f64_to_wrapping_i32(f64::parse(point_split.next().unwrap(), line_num)?);
                        let y =
                            f64_to_wrapping_i32(f64::parse(point_split.next().unwrap(), line_num)?);
                        control_points.push(Vector2::new(x as f32, y as f32));
                    }
                }

                slider_info.slides = std::cmp::max(1, i32::parse(split.next().unwrap(), line_num)?);
                if split_num > 7 {
                    slider_info.spatial_length = f64::parse(split.next().unwrap(), line_num)?;
                }

                slider_info.curve = Curve::new(
                    curve_type,
                    control_points,
                    self.format_version,
                    slider_info.spatial_length,
                );

                new_obj.unstacked_end_pos = slider_info.curve.point_at(1.0);
                new_obj.slider_info = Some(Box::new(slider_info));
            }
        }

        // sorted insert
        let pos = self
            .hit_objects
            .binary_search(&new_obj)
            .unwrap_or_else(|x| x);
        self.hit_objects.insert(pos, new_obj);
        //self.hit_objects.push(new_obj);

        Ok(())
    }

    // returns true if eof
    fn next_line(file: &mut impl BufRead, buffer: &mut String) -> Result<bool, BeatmapParseErr> {
        buffer.clear();
        Ok(file.read_line(buffer)? == 0)
    }

    fn split_key_val(source: &str) -> Option<(&str, &str)> {
        if let Some(sep_idx) = source.find(':') {
            let key = &source[0..sep_idx].trim();
            let val = &source[(sep_idx + 1)..source.len()].trim();
            Some((key, val))
        } else {
            None
        }
    }

    fn process_stacking(&mut self) {
        // TODO: this needs a test
        // port from https://github.com/ppy/osu/blob/master/osu.Game.Rulesets.Osu/Beatmaps/OsuBeatmapProcessor.cs
        if self.format_version >= 6 {
            // modern algorithm
            // there's a loop that gets hit if endIndex < beatmap.HitObjects.Count - 1 here, but endIndex is always the max in this
            let mut extended_start_idx = 0;
            for i in (0..self.hit_objects.len()).rev() {
                let objs = &mut self.hit_objects;
                let mut n = i;
                let mut obj_i_idx = i;

                if objs[obj_i_idx].stack_count != 0
                    || objs[obj_i_idx].object_type == HitObjectType::Spinner
                {
                    continue;
                }

                let stack_threshold = (self.difficulty.preempt as f32 * self.stack_leniency) as i32;

                match objs[obj_i_idx].object_type {
                    HitObjectType::Circle => {
                        while n != 0 {
                            n -= 1;

                            if objs[n].object_type == HitObjectType::Spinner {
                                continue;
                            }
                            if objs[obj_i_idx].start - objs[n].end > stack_threshold {
                                break;
                            }

                            if n < extended_start_idx {
                                objs[n].stack_count = 0;
                                extended_start_idx = n;
                            }

                            if objs[n].object_type == HitObjectType::Slider
                                && objs[n]
                                    .unstacked_end_pos
                                    .distance(objs[obj_i_idx].unstacked_start_pos)
                                    < 3.0
                            {
                                let offset = objs[obj_i_idx].stack_count - objs[n].stack_count + 1;

                                for j in (n + 1)..=obj_i_idx {
                                    if objs[n]
                                        .unstacked_end_pos
                                        .distance(objs[j].unstacked_start_pos)
                                        < 3.0
                                    {
                                        objs[j].stack_count -= offset;
                                    }
                                }

                                break;
                            }

                            if objs[n]
                                .unstacked_start_pos
                                .distance(objs[obj_i_idx].unstacked_start_pos)
                                < 3.0
                            {
                                objs[n].stack_count = objs[obj_i_idx].stack_count + 1;
                                obj_i_idx = n;
                            }
                        }
                    }
                    HitObjectType::Slider => {
                        while n != 0 {
                            n -= 1;

                            if objs[n].object_type == HitObjectType::Spinner {
                                continue;
                            }
                            if objs[obj_i_idx].start - objs[n].start > stack_threshold {
                                break;
                            }

                            if objs[n]
                                .unstacked_end_pos
                                .distance(objs[obj_i_idx].unstacked_start_pos)
                                < 3.0
                            {
                                objs[n].stack_count = objs[obj_i_idx].stack_count + 1;
                                obj_i_idx = n;
                            }
                        }
                    }
                    HitObjectType::Spinner => (),
                }
            }
        } else {
            // old algorithm, much simpler but doesn't handle as many special cases
            let stack_threshold = (self.difficulty.preempt as f32 * self.stack_leniency) as i32;
            let objs = &mut self.hit_objects;
            for i in 0..objs.len() {
                if objs[i].stack_count != 0 && objs[i].object_type != HitObjectType::Slider {
                    continue;
                }

                let mut start_time = objs[i].end;
                let mut slider_stack = 0;

                for j in (i + 1)..objs.len() {
                    if objs[j].start - stack_threshold > start_time {
                        break;
                    }

                    let pos2 = objs[i].unstacked_end_pos;

                    if objs[j]
                        .unstacked_start_pos
                        .distance(objs[i].unstacked_start_pos)
                        < 3.0
                    {
                        objs[i].stack_count += 1;
                        start_time = objs[j].end;
                    } else if objs[j].unstacked_start_pos.distance(pos2) < 3.0 {
                        slider_stack += 1;
                        objs[j].stack_count -= slider_stack;
                        start_time = objs[j].end;
                    }
                }
            }
        }

        for x in self.hit_objects.iter_mut() {
            let offset = Vector2::new(self.difficulty.stack_offset, self.difficulty.stack_offset)
                * x.stack_count as f32;
            x.start_pos = x.unstacked_start_pos - offset;
            x.end_pos = x.unstacked_end_pos - offset;
            x.stack_offset = offset;
        }
    }
}
