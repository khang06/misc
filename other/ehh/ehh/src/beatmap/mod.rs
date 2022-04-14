mod difficulty;
mod hitobject;
mod parser;
mod timing_point;

pub use difficulty::*;
pub use hitobject::*;
pub use parser::*;
pub use timing_point::*;

// TODO: default on its own doesn't get everything right
#[derive(Default)]
pub struct Beatmap {
    // required to get the actual paths of other files used by the beatmap
    pub base_path: String,

    // top of the header
    pub format_version: i32,

    // General
    pub always_show_playfield: bool,
    pub audio_filename: String,
    pub audio_hash: String, // currently unused in stable, but exists in old files
    pub audio_lead_in: i32,
    pub countdown: Countdown,
    pub countdown_offset: i32,
    pub custom_samples: bool,
    pub epilepsy_warning: bool,
    pub letterbox_in_breaks: bool,
    pub mode: Gamemode,
    pub overlay_position: OverlayPosition,
    pub preview_time: i32,
    pub sample_set: SampleSet,
    pub sample_volume: i32,
    pub samples_match_playback_rate: bool,
    pub skin_preference: String,
    pub special_style: bool, // mania only
    pub stack_leniency: f32, // editor only
    pub timeline_zoom: f32,  // editor only
    pub widescreen_storyboard: bool,

    // Metadata
    pub artist: String, // "ArtistUnicode"
    pub beatmap_id: i32,
    pub beatmap_set_id: i32,
    pub creator: String,
    pub romanized_artist: String, // "Artist"
    pub romanized_title: String,  // "Title"
    pub source: String,
    pub tags: String,    // TODO: should this automatically make a vec?
    pub title: String,   // "TitleUnicode"
    pub version: String, // or diff

    // Difficulty
    pub difficulty: Difficulty,

    // TODO: Events

    // TimingPoints
    pub timing_points: Vec<TimingPoint>,

    // TODO: Colours (british spelling is important)

    // HitObjects
    pub hit_objects: Vec<HitObject>,
    pub circle_count: usize,
    pub slider_count: usize,
    pub spinner_count: usize,
}

#[derive(Debug)]
pub enum Countdown {
    None = 0,
    Normal,
    HalfTime,
    DoubleTime,
}
impl Default for Countdown {
    fn default() -> Self {
        Countdown::Normal
    }
}

#[derive(Debug)]
pub enum Gamemode {
    Osu = 0,
    Taiko,
    CatchTheBeat,
    Mania,
}
impl Default for Gamemode {
    fn default() -> Self {
        Gamemode::Osu
    }
}

#[derive(Debug)]
pub enum OverlayPosition {
    NoChange = 0,
    Below,
    Above,
}
impl BeatmapParse for OverlayPosition {
    fn parse(source: &str, line_num: u32) -> Result<Self, BeatmapParseErr> {
        // parsed as case insensitive for some reason
        match &*source.to_lowercase() {
            "nochange" => Ok(OverlayPosition::NoChange),
            "below" => Ok(OverlayPosition::Below),
            "above" => Ok(OverlayPosition::Above),
            _ => Err(BeatmapParseErr::InvalidEnum(line_num)),
        }
    }
}
impl Default for OverlayPosition {
    fn default() -> Self {
        OverlayPosition::NoChange
    }
}
