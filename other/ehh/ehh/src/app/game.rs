use std::{cell::RefCell, fs::File, io::BufReader, path::PathBuf, rc::Rc};

use log::info;

use crate::{
    app::{audio_manager::AudioManager, hitobject_manager::HitObjectManager},
    framework::{
        bass::Bass,
        render::{Alignment, DrawBatch, TextRenderer, TextSprite},
    },
    Beatmap,
};

use super::asset_loader::AssetLoader;

// native osu ui resolution is 1024x768
// playfield resolution is 512x384, exactly half
pub const OSU_NATIVE_WIDTH: u32 = 1024;
pub const OSU_NATIVE_HEIGHT: u32 = 768;
pub const OSU_NATIVE_WIDESCREEN_EXTRA: u32 = 171;
pub const OSU_NATIVE_TO_PLAYFIELD_RATIO: u32 = 2;
pub const OSU_PLAYFIELD_WIDTH: u32 = OSU_NATIVE_WIDTH / OSU_NATIVE_TO_PLAYFIELD_RATIO;
pub const OSU_PLAYFIELD_HEIGHT: u32 = OSU_NATIVE_HEIGHT / OSU_NATIVE_TO_PLAYFIELD_RATIO;

struct OsuHUD {
    hitobject_manager: Rc<RefCell<HitObjectManager>>,
    text_renderer: Rc<RefCell<TextRenderer>>,
    batch: DrawBatch,
    text: TextSprite,
}

impl OsuHUD {
    pub fn new(
        width: f32,
        height: f32,
        hitobject_manager: Rc<RefCell<HitObjectManager>>,
        text_renderer: Rc<RefCell<TextRenderer>>,
    ) -> OsuHUD {
        let ortho = cgmath::ortho(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);
        let text = TextSprite::new(
            text_renderer.clone(),
            "Hellooooo Wooorld",
            8.0,
            0.0,
            0.25,
            Alignment::Left,
        );

        OsuHUD {
            hitobject_manager,
            text_renderer,
            batch: DrawBatch::new(ortho),
            text,
        }
    }

    pub fn update(&mut self) {}

    pub fn draw(&mut self) {
        self.text.set_text(&format!(
            "Visible objects: {}",
            self.hitobject_manager.borrow().visible_objs_count()
        ));
        self.text.add_to_batch(&mut self.batch);

        self.batch.draw();
    }
}

pub struct OsuGame {
    bass: Rc<Bass>,
    asset_loader: Rc<RefCell<AssetLoader>>,
    audio_manager: Rc<RefCell<AudioManager>>,
    hitobject_manager: Rc<RefCell<HitObjectManager>>,
    text_renderer: Rc<RefCell<TextRenderer>>,

    hud: OsuHUD,

    width: f32,
    height: f32,
}

impl OsuGame {
    pub fn new(
        bass: Rc<Bass>,
        text_renderer: Rc<RefCell<TextRenderer>>,
        width: f32,
        height: f32,
        beatmap_path: String,
    ) -> Result<OsuGame, String> {
        info!("Opening {}...", beatmap_path);
        let beatmap_file = match File::open(beatmap_path.clone()) {
            Ok(x) => x,
            Err(_) => {
                return Err("Failed to open beatmap".to_string());
            }
        };
        let mut folder = PathBuf::from(beatmap_path);
        folder.pop();
        let beatmap =
            match Beatmap::parse(&folder.to_string_lossy(), &mut BufReader::new(beatmap_file)) {
                Ok(x) => x,
                Err(_) => {
                    return Err("Failed to parse beatmap".to_string());
                }
            };

        // TODO: don't hardcode this path when i get config stuff implemented
        let asset_loader = Rc::new(RefCell::new(AssetLoader::new(
            "C:\\Users\\Khang\\AppData\\Local\\osu!\\Skins\\Luminous",
            //"F:\\osu!\\skins\\Awesome's Clear Skin v10",
        )));

        let audio_manager = Rc::new(RefCell::new(AudioManager::new(
            bass.clone(),
            asset_loader.clone(),
            &format!("{}/{}", beatmap.base_path, beatmap.audio_filename),
        )));
        // TODO: properly dynamically set the lead in based on storyboard events and stuff
        audio_manager
            .borrow_mut()
            .seek_music(beatmap.hit_objects[0].start as f64 - 1800.0);
        audio_manager.borrow_mut().resume_music();

        let hitobject_manager = Rc::new(RefCell::new(HitObjectManager::new(
            width,
            height,
            asset_loader.clone(),
            audio_manager.clone(),
            Rc::new(beatmap),
        )));

        let hud = OsuHUD::new(
            width,
            height,
            hitobject_manager.clone(),
            text_renderer.clone(),
        );

        Ok(OsuGame {
            bass,
            asset_loader,
            audio_manager,
            hitobject_manager,
            text_renderer,
            hud,
            width,
            height,
        })
    }

    pub fn get_title(&self) -> String {
        let beatmap = &self.hitobject_manager.borrow().beatmap;
        format!(
            "ehh | {} - {} [{}]",
            beatmap.romanized_artist, beatmap.romanized_title, beatmap.version
        )
    }

    pub fn update(&mut self) {
        self.audio_manager.borrow_mut().update();

        let audio_time = self.audio_manager.borrow().music_pos() as i32;
        self.hitobject_manager.borrow_mut().update(audio_time);

        self.hud.update();
    }

    pub fn draw(&mut self) {
        let audio_time = self.audio_manager.borrow().music_pos() as i32;
        self.hitobject_manager.borrow_mut().draw(audio_time);

        self.hud.draw();
    }
}
