use std::{fs::File, io::BufReader, path::PathBuf, rc::Rc, time::Instant};

use clap::{Parser, Subcommand};
use ehh::{
    app::EhhApp,
    framework::bass::{Bass, BassChannelCommon},
    Beatmap,
};
use log::{error, info};

#[rustfmt::skip]
fn dump_beatmap_info(beatmap: Beatmap) {
    println!("Format version: {}", beatmap.format_version);

    println!("General:");
    println!("    Always Show Playfield:       {}",   beatmap.always_show_playfield);
    println!("    Audio Filename:              {}",   beatmap.audio_filename);
    println!("    Audio Hash:                  {}",   beatmap.audio_hash);
    println!("    Audio Lead-in:               {}",   beatmap.audio_lead_in);
    println!("    Countdown:                   {:?}", beatmap.countdown);
    println!("    Countdown Offset:            {}",   beatmap.countdown_offset);
    println!("    Custom Samples:              {}",   beatmap.custom_samples);
    println!("    Epilespy Warning:            {}",   beatmap.epilepsy_warning);
    println!("    Letterbox in Breaks:         {}",   beatmap.letterbox_in_breaks);
    println!("    Mode:                        {:?}", beatmap.mode);
    println!("    Overlay Position:            {:?}", beatmap.overlay_position);
    println!("    Preview Time:                {}",   beatmap.preview_time);
    println!("    Sample Set:                  {:?}", beatmap.sample_set);
    println!("    Sample Volume:               {}",   beatmap.sample_volume);
    println!("    Samples Match Playback Rate: {}",   beatmap.samples_match_playback_rate);
    println!("    Skin Preference:             {}",   beatmap.skin_preference);
    println!("    Special Style:               {}",   beatmap.special_style);
    println!("    Stack Leniency:              {}",   beatmap.stack_leniency);
    println!("    Timeline Zoom:               {}",   beatmap.timeline_zoom);
    println!("    Widescreen Storyboard:       {}",   beatmap.widescreen_storyboard);

    println!("Metadata:");
    println!("    Artist:           {}", beatmap.artist);
    println!("    Romanized Artist: {}", beatmap.romanized_artist);
    println!("    Beatmap ID:       {}", beatmap.beatmap_id);
    println!("    Beatmap Set ID:   {}", beatmap.beatmap_set_id);
    println!("    Creator:          {}", beatmap.creator);
    println!("    Source:           {}", beatmap.source);
    println!("    Tags:             {}", beatmap.tags);
    println!("    Title:            {}", beatmap.title);
    println!("    Romanized Title:  {}", beatmap.romanized_title);
    println!("    Version:          {}", beatmap.version);

    println!("Difficulty:");
    println!("    Approach Rate:      {}",   beatmap.difficulty.approach_rate);
    println!("        Preempt:        {}ms", beatmap.difficulty.preempt);
    println!("    Circle Size:        {}",   beatmap.difficulty.circle_size);
    println!("    HP Drain:           {}",   beatmap.difficulty.hp_drain);
    println!("    Overall Difficulty: {}",   beatmap.difficulty.overall_difficulty);
    println!("        300 window:     {}ms", beatmap.difficulty.hit_300);
    println!("        100 window:     {}ms", beatmap.difficulty.hit_100);
    println!("        50 window:      {}ms", beatmap.difficulty.hit_50);
    println!("    Slider Multiplier:  {}",   beatmap.difficulty.slider_multiplier);
    println!("    Slider Tick Rate:   {}",   beatmap.difficulty.slider_tick_rate);

    /*
    println!("Timing Points:");
    let tp_len = beatmap.timing_points.len();
    println!("{} timing points", tp_len);
    for x in beatmap.timing_points {
        println!("    {}, {}", x.offset, if !x.timing_change {format!("{}x", 100.0 / -x.beat_length)} else {format!("{} bpm", 60000.0 / x.beat_length)});
    }
    */

    println!("Max combo: {}", {
        let mut ret: usize = 0;
        for x in beatmap.hit_objects {
            ret += 1;
            if let Some(slider_info) = x.slider_info {
                ret += slider_info.score_times.len();
            }
        }
        ret
    });
    println!(
        "{} circles, {} sliders, {} spinners",
        beatmap.circle_count, beatmap.slider_count, beatmap.spinner_count
    );
}

#[derive(Parser)]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Parse { beatmap: Option<String> },
    BatchParse { beatmap_dir: Option<String> },
    TestBass { song: Option<String> },
    Play { beatmap: Option<String> },
}

fn parse_map(path: &str) -> Result<(), std::io::Error> {
    println!("Parsing {path}...");
    let mut folder = PathBuf::from(path);
    folder.pop();
    let res = Beatmap::parse(
        &folder.to_string_lossy(),
        &mut BufReader::new(File::open(path)?),
    );

    if let Ok(beatmap) = res {
        dump_beatmap_info(beatmap);
    } else {
        println!("Failed to parse the beatmap!");
    }

    Ok(())
}

fn batch_parse_maps(path: &str) {
    let ext = std::ffi::OsStr::new("osu");
    let mut parsed = 0;
    let mut skipped = 0;
    let mut errored = 0;
    let now = Instant::now();
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension() != Some(ext) {
            continue;
        }

        println!("Parsing {}...", entry.file_name().to_str().unwrap());
        let mut folder = entry.path().to_path_buf();
        folder.pop();
        let beatmap = Beatmap::parse(
            &folder.to_string_lossy(),
            &mut BufReader::with_capacity(100_000, File::open(path).unwrap()),
        );

        if beatmap.is_err() {
            let err = beatmap.err().unwrap();
            if !matches!(err, ehh::BeatmapParseErr::UnsupportedMode) {
                error!("Failed to parse {:?} because {:?}", path, err);
                errored += 1;
            } else {
                skipped += 1;
            }
        }
        parsed += 1;
        // if parsed % 100 == 0 {
        //     println!("Parsed {} beatmaps", parsed);
        // }
    }
    let elapsed_sec = now.elapsed().as_millis() as f64 / 1000.0;
    println!("Parsed {} beatmaps in {} seconds ({} beatmaps/sec, errored on {}, and skipped {} due to unsupported mode)",
        parsed, elapsed_sec, parsed as f64 / elapsed_sec, errored, skipped);
}

fn test_bass(path: &str) {
    let bass = Bass::new(-1, 44100, 0).unwrap();
    /*
    for i in 1..u32::MAX {
        match bass.get_device_info(i) {
            Ok(device) => info!("Device: {}", device.name),
            Err(_) => break
        }
    }
    */

    let bass_version = bass.get_version();
    let bassmix_version = bass.get_bassmix_version();
    info!(
        "BASS version:    {}.{}.{}.{}",
        bass_version.0, bass_version.1, bass_version.2, bass_version.3
    );
    info!(
        "BASSmix version: {}.{}.{}.{}",
        bassmix_version.0, bassmix_version.1, bassmix_version.2, bassmix_version.3
    );
    info!(
        "Using device:    {}",
        bass.get_device_info(bass.get_device().unwrap())
            .unwrap()
            .name
    );

    let mixer = Rc::new(
        bass.create_mixer(
            44100,
            2,
            bassmix_sys::BASS_MIXER_NONSTOP | bass_sys::BASS_SAMPLE_FLOAT,
        )
        .unwrap(),
    );
    mixer.set_attrib(bass_sys::BASS_ATTRIB_BUFFER, 0.0);
    mixer.set_attrib(bass_sys::BASS_ATTRIB_VOL, 0.5);
    mixer.set_device(bass.get_device().unwrap()).unwrap();
    mixer.play(false).unwrap();

    let stream = bass
        .create_stream_from_file(path, bass_sys::BASS_STREAM_DECODE)
        .unwrap();
    mixer.add_channel(stream, 0).unwrap();

    info!("Waiting 2 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let sample = bass
        .create_sample_from_file(
            Some(mixer.clone()),
            "c:\\Users\\Khang\\AppData\\Local\\osu!\\Skins\\Luminous\\combobreak.wav",
            8,
            0,
        )
        .unwrap();
    for _ in 0..100 {
        let chan = sample
            .get_channel(
                bass_sys::BASS_SAMCHAN_STREAM | bass_sys::BASS_STREAM_DECODE,
                true,
            )
            .unwrap();
        mixer
            .add_channel(
                chan,
                bassmix_sys::BASS_MIXER_CHAN_NORAMPIN | bass_sys::BASS_STREAM_AUTOFREE,
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    info!("Waiting 2 more seconds...");
    std::thread::sleep(std::time::Duration::from_secs(2));
}

fn main() -> Result<(), std::io::Error> {
    let cli = Cli::parse();

    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    match &cli.command {
        Commands::Parse { beatmap } => {
            if let Some(filename) = beatmap.as_ref() {
                parse_map(filename).unwrap();
            } else {
                println!("You must specify a beatmap path!");
            }
        }
        Commands::BatchParse { beatmap_dir } => {
            if let Some(dir) = beatmap_dir.as_ref() {
                batch_parse_maps(dir);
            } else {
                println!("You must specify a beatmap folder!");
            }
        }
        Commands::TestBass { song } => {
            if let Some(song) = song.as_ref() {
                test_bass(song);
            } else {
                println!("You must specify a song path!");
            }
        }
        Commands::Play { beatmap } => {
            if let Some(filename) = beatmap.as_ref() {
                EhhApp::run(filename.clone());
            } else {
                println!("You must specify a beatmap path!");
            }
        }
    }

    Ok(())
}
