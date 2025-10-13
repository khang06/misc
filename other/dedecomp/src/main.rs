use std::{collections::{HashMap, HashSet}, fs::File, io::{BufWriter, Cursor, Read, Write}};

use byteorder::{BigEndian, ReadBytesExt};
use common::{SegmentBase, SegmentData, SegmentState, SegmentedPointer, SymbolState};
use geo::GeoLayout;
use gfx::Gfx;
use level::LevelScript;
use ram_syms::RAM_SYMS;

mod collision;
mod common;
mod level;
mod geo;
mod gfx;
mod ram_syms;

struct Level {
    pub id: u32,
    pub folder_name: &'static str,
    pub script_exec_offset: u32,
    pub seg_e_base: Option<u32>,
}

impl Level {
    const fn new(id: u32, folder_name: &'static str, script_exec_offset: u32, seg_e_base: Option<u32>) -> Self {
        Self {
            id,
            folder_name,
            script_exec_offset,
            seg_e_base,
        }
    }
}

struct ScriptWorklist {
    worklist: Vec<(SegmentedPointer, SegmentState)>,
    processed: HashSet<SegmentedPointer>,
}

impl ScriptWorklist {
    pub fn new(root: Option<(SegmentedPointer, SegmentState)>, sym: &SymbolState) -> Self {
        let mut worklist = vec![];
        let mut processed = sym.get_skip_addrs();
        if let Some(root) = root {
            processed.insert(root.0);
            worklist.push(root);
        }
        Self {
            worklist,
            processed,
        }
    }

    pub fn push(&mut self, addr: SegmentedPointer, seg_state: &SegmentState) {
        if !matches!(addr, SegmentedPointer::Ram(0)) && self.processed.insert(addr) {
            self.worklist.push((addr, seg_state.clone()));
        }
    }

    pub fn push_raw(&mut self, addr: u32, seg_state: &SegmentState) {
        self.push(seg_state.addr_to_seg_ptr(addr), seg_state)
    }

    pub fn pop(&mut self) -> Option<(SegmentedPointer, SegmentState)> {
        self.worklist.pop()
    }
}

#[derive(Default)]
struct RawDataWorklist<T> {
    worklist: Vec<(SegmentedPointer, T)>,
    processed: HashSet<SegmentedPointer>,
}

impl<T: Copy> RawDataWorklist<T> {
    pub fn push(&mut self, addr: SegmentedPointer, metadata: T) {
        if !matches!(addr, SegmentedPointer::Ram(0)) && self.processed.insert(addr) {
            self.worklist.push((addr, metadata));
        }
    }

    pub fn into_vec(self) -> Vec<(SegmentedPointer, T)> {
        self.worklist
    }
}

const IM_FMT_RGBA: u8 = 0;
// const IM_FMT_YUV: u8 = 1; // who cares
const IM_FMT_CI: u8 = 2;
const IM_FMT_IA: u8 = 3;
const IM_FMT_I: u8 = 4;

const IM_SIZ_4B: u8 = 0;
const IM_SIZ_8B: u8 = 1;
const IM_SIZ_16B: u8 = 2;
const IM_SIZ_32B: u8 = 3;

struct TextureData {
    pub addr: SegmentedPointer,
    pub dim: Option<(u16, u16)>,
    pub format: u8,
    pub size: u8,
    pub palette: Option<SegmentedPointer>
}

fn rgba5551to8888(col: u16) -> (u8, u8, u8, u8) {
    let scale = |x: u8| ((x as u32 * 527 + 23) >> 6) as u8;

    let r = scale((col >> 11) as u8 & 0x1F);
    let g = scale((col >> 6) as u8 & 0x1F);
    let b = scale((col >> 1) as u8 & 0x1F);
    let a = if (col & 1) != 0 { 0xFF } else { 0 };

    (r, g, b, a)
}

impl TextureData {
    pub fn as_rgba32(&self, seg_data: &SegmentData) -> Result<Vec<u8>, std::io::Error> {
        let dim = {
            let ret = self.dim.unwrap_or_else(|| panic!("can't convert without dimensions"));
            (ret.0 as usize, ret.1 as usize)
        };

        let mut reader = seg_data.get_reader_from_seg_ptr(self.addr)?;

        let mut src = vec![0; match self.size {
            IM_SIZ_4B => dim.0 * dim.1 / 2,
            IM_SIZ_8B => dim.0 * dim.1,
            IM_SIZ_16B => dim.0 * dim.1 * 2,
            IM_SIZ_32B => dim.0 * dim.1 * 4,
            x => unimplemented!("Unhandled tex size {x:#X} for {:?}", self.addr)
        } as usize];
        reader.read_exact(&mut src)?;

        let mut dst = vec![0u8; dim.0 * dim.1 * 4];
        match (self.format, self.size) {
            (IM_FMT_RGBA, IM_SIZ_16B) => {
                for (i, arr) in src.chunks_exact(2).enumerate() {
                    let col = ((arr[0] as u16) << 8) | arr[1] as u16;
                    let (r, g, b, a) = rgba5551to8888(col);
                    dst[i * 4] = r;
                    dst[i * 4 + 1] = g;
                    dst[i * 4 + 2] = b;
                    dst[i * 4 + 3] = a;
                }
            },
            (IM_FMT_RGBA, IM_SIZ_32B) => dst.copy_from_slice(&src),

            (IM_FMT_CI, IM_SIZ_4B) => {
                let mut lut = [0u8; 16 * 2];
                let mut lut_reader = seg_data.get_reader_from_seg_ptr(self.palette.unwrap())?;
                lut_reader.read_exact(&mut lut)?;

                for i in 0..src.len() {
                    let idx1 = (src[i] >> 4) as usize * 2;
                    let col1 = ((lut[idx1] as u16) << 8) | lut[idx1 + 1] as u16;
                    let (r1, g1, b1, a1) = rgba5551to8888(col1);
                    dst[i * 8] = r1;
                    dst[i * 8 + 1] = g1;
                    dst[i * 8 + 2] = b1;
                    dst[i * 8 + 3] = a1;

                    let idx2 = (src[i] & 0xF) as usize * 2;
                    let col2 = ((lut[idx2] as u16) << 8) | lut[idx2 + 1] as u16;
                    let (r2, g2, b2, a2) = rgba5551to8888(col2);
                    dst[i * 8 + 4] = r2;
                    dst[i * 8 + 5] = g2;
                    dst[i * 8 + 6] = b2;
                    dst[i * 8 + 7] = a2;
                }
            }
            (IM_FMT_CI, IM_SIZ_8B) => {
                // NOTE: Not all CI8 textures use all 256 colors, but this might be fine
                let mut lut = [0u8; 256 * 2];
                let mut lut_reader = seg_data.get_reader_from_seg_ptr(self.palette.unwrap())?;
                lut_reader.read_exact(&mut lut)?;

                for i in 0..src.len() {
                    let idx = src[i] as usize * 2;
                    let col = ((lut[idx] as u16) << 8) | lut[idx + 1] as u16;
                    let (r, g, b, a) = rgba5551to8888(col);
                    dst[i * 4] = r;
                    dst[i * 4 + 1] = g;
                    dst[i * 4 + 2] = b;
                    dst[i * 4 + 3] = a;
                }
            }

            (IM_FMT_IA, IM_SIZ_4B) => {
                for i in 0..src.len() {
                    let col1 = ((src[i] >> 5) * 146 + 1) >> 2;
                    let alpha1 = if (src[i] & 0x10) != 0 { 0xFF } else { 0 };
                    dst[i * 8] = col1;
                    dst[i * 8 + 1] = col1;
                    dst[i * 8 + 2] = col1;
                    dst[i * 8 + 3] = alpha1;

                    let col2 = ((src[i] >> 1) * 146 + 1) >> 2;
                    let alpha2 = if (src[i] & 1) != 0 { 0xFF } else { 0 };
                    dst[i * 8 + 4] = col2;
                    dst[i * 8 + 5] = col2;
                    dst[i * 8 + 6] = col2;
                    dst[i * 8 + 7] = alpha2;
                }
            }
            (IM_FMT_IA, IM_SIZ_8B) => {
                for i in 0..src.len() {
                    let col = (src[i] >> 4) * 17;
                    let alpha = (src[i] & 0xF) * 17;

                    dst[i * 4] = col;
                    dst[i * 4 + 1] = col;
                    dst[i * 4 + 2] = col;
                    dst[i * 4 + 3] = alpha;
                }
            }
            (IM_FMT_IA, IM_SIZ_16B) => {
                for (i, arr) in src.chunks_exact(2).enumerate() {
                    let col = arr[0];
                    let alpha = arr[1];
                    dst[i * 4] = col;
                    dst[i * 4 + 1] = col;
                    dst[i * 4 + 2] = col;
                    dst[i * 4 + 3] = alpha;
                }
            }

            (IM_FMT_I, IM_SIZ_4B) => {
                for i in 0..src.len() {
                    let col1 = (src[i] >> 4) * 17;
                    dst[i * 8] = col1;
                    dst[i * 8 + 1] = col1;
                    dst[i * 8 + 2] = col1;
                    dst[i * 8 + 3] = 0xFF;

                    let col2 = (src[i] & 0xF) * 17;
                    dst[i * 8 + 4] = (col2 << 4) | col2;
                    dst[i * 8 + 5] = (col2 << 4) | col2;
                    dst[i * 8 + 6] = (col2 << 4) | col2;
                    dst[i * 8 + 7] = 0xFF;
                }
            }
            (IM_FMT_I, IM_SIZ_8B) | (IM_FMT_I, IM_SIZ_16B) => {
                for i in 0..src.len() {
                    let col = src[i];
                    dst[i * 4] = col;
                    dst[i * 4 + 1] = col;
                    dst[i * 4 + 2] = col;
                    dst[i * 4 + 3] = 0xFF;
                }
            }
            (f, s) => unimplemented!("unhandled format {f} with size {s} for {:?}", self.addr)
        }
        Ok(dst)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BASE_PATH: &str = "D:\\Github\\sm64coopdx\\build\\us_pc\\mods\\serene-fusion";

    const LEVELS: &[Level] = &[
        Level::new(0x4, "bbh", 0xB28, None),
        Level::new(0x5, "ccm", 0xB0C, None),
        Level::new(0x6, "castle_inside", 0xAF0, Some(0x301D40)), // greyscale castle
        Level::new(0x7, "hmc", 0xAD4, None),
        Level::new(0x8, "ssl", 0xAB8, None),
        Level::new(0x9, "bob", 0xA9C, Some(0x3453C0)), // abandoned catacombs
        Level::new(0xA, "sl", 0xA80, None),
        Level::new(0xB, "wdw", 0xA64, None),
        Level::new(0xC, "jrb", 0xA48, None),
        Level::new(0xD, "thi", 0xA2C, None), // bowser's diorama fortress (duplicate??????)
        Level::new(0xE, "ttc", 0xA10, None),
        Level::new(0xF, "rr", 0x9F4, None),
        Level::new(0x10, "castle_grounds", 0x9D8, Some(0x3E7070)), // main lobby
        Level::new(0x11, "bitdw", 0x9BC, None),
        Level::new(0x12, "vcutm", 0x9A0, Some(0x42D890)), // palace of earth spirits
        Level::new(0x13, "bitfs", 0x984, None),
        Level::new(0x14, "sa", 0x968, Some(0x454740)), // dual rush
        Level::new(0x15, "bits", 0x94C, None),
        Level::new(0x16, "lll", 0x930, None),
        Level::new(0x17, "ddd", 0x914, None),
        Level::new(0x18, "wf", 0x8F8, None),
        Level::new(0x19, "ending", 0x8DC, None),
        Level::new(0x1A, "castle_courtyard", 0x8C0, Some(0x496BD0)), // black screen lobby pipe entrance thing
        Level::new(0x1B, "pss", 0x8A4, Some(0x4D82D0)), // sandy hill slide
        Level::new(0x1C, "cotmc", 0x888, Some(0x4EB950)), // ancient gear tower
        Level::new(0x1D, "totwc", 0x86C, Some(0x4F67D0)), // flipnote studio
        Level::new(0x1E, "bowser_1", 0x850, None),
        Level::new(0x1F, "wmotr", 0x834, Some(0x5153D0)), // grain silo ruins
        Level::new(0x21, "bowser_2", 0x818, Some(0x520C70)), // the final fight (bowser)
        Level::new(0x22, "bowser_3", 0x7FC, None),
        Level::new(0x24, "ttm", 0x7E0, None),
        Level::new(0x27, "ab", 0x7C4, Some(0x574A30)), // azure abyss
        Level::new(0x28, "df", 0x7A8, Some(0x592B30)), // dice fortress
        Level::new(0x29, "mf", 0x78C, Some(0x5C20F0)), // flower fields
        Level::new(0x2A, "mtc", 0x770, Some(0x5F5C00)), // molten treasure chest
        Level::new(0x2B, "hf", 0x754, Some(0x6215D0)), // haunted factory
        Level::new(0x2C, "vcm", 0x738, Some(0x63BE90)), // predestined fate
        Level::new(0x2D, "bdf", 0x71C, Some(0x693F70)), // bowser's diorama fortress
        Level::new(0x2E, "crash", 0x700, Some(0x6A6E30)), // ashellerated automata
        Level::new(0x2F, "spiders", 0x6E4, Some(0x6DEFC0)), // honeyhive falls
        Level::new(0x30, "rovert", 0x6C8, Some(0x717B50)), // time travel ruins
        Level::new(0x31, "rng", 0x6AC, Some(0x737AE0)), // star gazed medley
        Level::new(0x32, "luigiman", 0x690, Some(0x74D9C0)), // dead space base
        Level::new(0x33, "dan", 0x674, Some(0x7577D0)), // slip on a banana peel
        Level::new(0x34, "sphere", 0x658, Some(0x75BFA0)), // golf sphere
    ];

    const SEGMENT_SEGMENT2_BASE: u32 = 0xEA990;
    const SEGMENT_SEGMENT2_END: u32 = 0xF41D0;
    const SEGMENT_LEVEL_ENTRY_BASE: u32 = 0xEA960;
    const SEGMENT_BEHAVIOR_DATA_BASE: u32 = 0x1D4EA0;
    const SEGMENT_GLOBAL_LEVEL_SCRIPT_BASE: u32 = 0x1ED1B0;
    const SEGMENT_COMMON1_GEO_BASE: u32 = 0x1D3F40;
    const SEGMENT_GROUP0_GEO_BASE: u32 = 0x10FA70;

    let mut seg_data = SegmentData::new(include_bytes!("E:\\Games\\Emulation\\rhdc-hacks\\pjsf-1.3.2 6342676617316e22c431f6f9.z64"));
    seg_data.load_mio0(SEGMENT_SEGMENT2_BASE, SEGMENT_SEGMENT2_END);

    let mut seg_state: SegmentState = Default::default();
    seg_state.set_seg_base(0x2, SegmentBase::MIO0(SEGMENT_SEGMENT2_BASE));
    seg_state.set_seg_base(0x10, SegmentBase::Raw(SEGMENT_LEVEL_ENTRY_BASE));
    seg_state.set_seg_base(0x15, SegmentBase::Raw(SEGMENT_GLOBAL_LEVEL_SCRIPT_BASE));
    /*
    seg_state.set_seg_base(0x13, SegmentBase::Raw(SEGMENT_BEHAVIOR_DATA_BASE));
    seg_state.set_seg_base(0x16, SegmentBase::Raw(SEGMENT_COMMON1_GEO_BASE));
    seg_state.set_seg_base(0x17, SegmentBase::Raw(SEGMENT_GROUP0_GEO_BASE));
    */

    let mut sym: SymbolState = Default::default();

    // Common behaviors
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_BEHAVIOR_DATA_BASE, 0x3C78), "bhvMario", true);

    // Global scripts
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GLOBAL_LEVEL_SCRIPT_BASE, 0x304), "script_func_global_1", true);

    // Native functions
    for (addr, name) in RAM_SYMS {
        sym.add_sym(SegmentedPointer::Ram(*addr), name, true);
    }

    // Script entry stuff
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_LEVEL_ENTRY_BASE, 0x0), "level_script_entry", false);
    sym.add_sym(SegmentedPointer::RomRaw(0x1E1E70, 0x220), "level_intro_splash_screen", false);
    sym.add_sym(SegmentedPointer::RomRaw(0x1E1E70, 0x1DC), "level_intro_mario_head_regular", false);
    sym.add_sym(SegmentedPointer::RomRaw(0x1E1E70, 0x94), "script_intro_file_select", false);
    sym.add_sym(SegmentedPointer::RomRaw(0x1E1E70, 0x68), "script_intro_level_select", false);
    sym.add_sym(SegmentedPointer::RomRaw(0x1E1E70, 0x30), "script_intro_main_level_entry", false);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GLOBAL_LEVEL_SCRIPT_BASE, 0x3CC), "level_main_scripts_entry", false);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GLOBAL_LEVEL_SCRIPT_BASE, 0xBBC), "script_exec_level_table", false);

    // Stuff that has to be reimplemented due to custom geo commands
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xE00), "yellow_coin_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xDE4), "yellow_coin_no_shadow_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xDC4), "blue_coin_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xDA8), "blue_coin_no_shadow_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xD88), "red_coin_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xD6C), "red_coin_no_shadow_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xD4C), "silver_coin_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xD30), "silver_coin_no_shadow_geo", true);

    // Geo stuff that (probably) didn't change
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x4260), "smoke_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x40D8), "sparkles_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x42C8), "bubble_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x41E4), "small_water_splash_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x41A0), "idle_water_wave_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x4084), "water_splash_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x414C), "wave_trail_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GROUP0_GEO_BASE, 0x4028), "sparkles_animation_geo", true);
    sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_COMMON1_GEO_BASE, 0xEB4), "explosion_geo", true);

    // script_exec_*
    for x in LEVELS {
        sym.add_sym(SegmentedPointer::RomRaw(SEGMENT_GLOBAL_LEVEL_SCRIPT_BASE, x.script_exec_offset), &format!("script_exec_{}", x.folder_name), x.seg_e_base.is_none());
    }

    // level_*_entry
    for x in LEVELS {
        if let Some(seg_e_base) = x.seg_e_base {
            sym.add_sym(SegmentedPointer::RomRaw(seg_e_base, 0), &format!("level_{}_entry", x.folder_name), false);
        }
    }

    let mut nodes = vec![];

    let mut level_scripts = ScriptWorklist::new(Some((seg_state.addr_to_seg_ptr(0x150003CC), seg_state)), &sym);
    let mut geos = ScriptWorklist::new(None, &sym);
    let mut collisions = ScriptWorklist::new(None, &sym);
    let mut gfx = ScriptWorklist::new(None, &sym);

    let mut vertex_buffers = RawDataWorklist::default();
    let mut lights = RawDataWorklist::default();
    let mut rooms = RawDataWorklist::default();

    let mut textures = vec![];
    let mut textures_processed: HashSet<SegmentedPointer> = sym.get_skip_addrs();

    let mut surface_counts: HashMap<SegmentedPointer, usize> = Default::default();

    println!("levels...");
    while let Some((addr, mut seg_state)) = level_scripts.pop() {
        let parsed = level::parse_script(&seg_data, addr)?;

        let mut cur_terrain = None;
        for x in parsed.iter() {
            match x {
                LevelScript::LoadMIO0(seg, start, end) | LevelScript::LoadMIO0Texture(seg, start, end) => {
                    seg_data.load_mio0(*start, *end);
                    seg_state.set_seg_base(*seg as usize, SegmentBase::MIO0(*start))
                },
                LevelScript::LoadRaw(seg, start, _end, _bss_start, _bss_end) => seg_state.set_seg_base(*seg as usize, SegmentBase::Raw(*start)),
                LevelScript::Jump(addr) | LevelScript::JumpIf(_, _, addr) | LevelScript::JumpLink(addr) => level_scripts.push_raw(*addr, &seg_state),
                LevelScript::Execute(seg, script, _, entry, _, _) | LevelScript::ExitAndExecute(seg, script, _, entry) => {
                    let mut temp = seg_state.clone();
                    temp.set_seg_base(*seg as usize, SegmentBase::Raw(*script));
                    level_scripts.push_raw(*entry, &temp);
                }
                LevelScript::LoadModelFromGeo(_, geo) | LevelScript::BeginArea(_, geo) => geos.push_raw(*geo, &seg_state),
                LevelScript::Terrain(terrain_data) => {
                    collisions.push_raw(*terrain_data, &seg_state);
                    cur_terrain = Some(seg_state.addr_to_seg_ptr(*terrain_data));
                }
                LevelScript::LoadModelFromDL(_, display_list, _) => gfx.push_raw(*display_list, &seg_state),
                LevelScript::Rooms(addr) => {
                    rooms.push(
                        seg_state.addr_to_seg_ptr(*addr),
                        cur_terrain.unwrap_or_else(|| panic!("room node {} has no terrain", sym.resolve_raw_ptr(&seg_state, *addr)))
                    );
                }
                _ => {}
            }
        }
        nodes.push(level::to_c(&parsed, &sym.resolve_seg_ptr(addr), &mut seg_state, &sym)?);
    }

    println!("geos...");
    while let Some((addr, seg_state)) = geos.pop() {
        println!("processing {}...", sym.resolve_seg_ptr(addr));
        let parsed = geo::parse_script(&seg_data, addr)?;
        for x in parsed.iter() {
            match x {
                GeoLayout::Branch(_, target) => geos.push_raw(*target, &seg_state),
                GeoLayout::DisplayList(_, display_list) |
                    GeoLayout::AnimatedPart(_, _, _, _, display_list) => gfx.push_raw(*display_list, &seg_state),
                GeoLayout::TranslateRotate(params, _, _, _, _, _, _, display_list) |
                    GeoLayout::Translate(params, _, _, _, display_list) | 
                    GeoLayout::Rotate(params, _, _, _, display_list) |
                    GeoLayout::Scale(params, _, display_list) => if (params & 0x80) != 0 {
                        gfx.push_raw(*display_list, &seg_state);
                    }
                _ => {}
            }
        }

        nodes.push(geo::to_c(&parsed, &sym.resolve_seg_ptr(addr), &seg_state, &sym)?);
    }

    println!("collisions...");
    while let Some((addr, _)) = collisions.pop() {
        let parsed = collision::parse_script(&seg_data, addr)?;
        surface_counts.insert(addr, parsed.tri_count());
        nodes.push(parsed.to_c(&sym.resolve_seg_ptr(addr))?);
    }

    // TODO: Properly resolve texture updates between calls
    let tex_dim_hack: HashMap<SegmentedPointer, (u16, u16)> = HashMap::from([
        (SegmentedPointer::RomMIO0(0x113D60, 0x10940), (32, 32)),
        (SegmentedPointer::RomMIO0(0x113D60, 0x11140), (32, 32)),
        (SegmentedPointer::RomMIO0(0x113D60, 0x11940), (32, 32)),
        (SegmentedPointer::RomMIO0(0x113D60, 0x12140), (32, 32)),
        (SegmentedPointer::RomMIO0(0x113D60, 0x12940), (32, 32)),
        (SegmentedPointer::RomMIO0(0x113D60, 0x13140), (32, 32)),
        (SegmentedPointer::RomMIO0(0x113D60, 0x13940), (32, 32)),
        (SegmentedPointer::RomMIO0(0x113D60, 0x14140), (32, 32)),

        (SegmentedPointer::RomMIO0(0x7190F0, 0x2AE0), (32, 32)),

        (SegmentedPointer::RomMIO0(0x1AC6D0, 0x108D0), (32, 32)),
        (SegmentedPointer::RomMIO0(0x1AC6D0, 0x110D0), (32, 32)),
        (SegmentedPointer::RomMIO0(0x1AC6D0, 0x118D0), (32, 32)),
        (SegmentedPointer::RomMIO0(0x1AC6D0, 0x120D0), (32, 32)),
    ]);

    println!("display lists...");
    while let Some((addr, seg_state)) = gfx.pop() {
        //writeln!(buf, "// processing {}...", sym.resolve_ptr(addr));
        let parsed = gfx::parse_script(&seg_data, addr)?;

        let mut cur_tex: Option<TextureData> = None;
        let mut palette: Option<TextureData> = None;

        for x in parsed.iter() {
            match x {
                Gfx::BranchList(display_list) | Gfx::DisplayList(display_list) => gfx.push_raw(*display_list, &seg_state),
                Gfx::Vertex(v, n, _) => vertex_buffers.push(seg_state.addr_to_seg_ptr(*v), *n as usize),
                Gfx::MoveMem(adrs, _, 0xA, _) => lights.push(seg_state.addr_to_seg_ptr(*adrs), ()),

                Gfx::SetTextureImage(f, s, _, i) => {
                    if let Some(tex) = cur_tex.take() {
                        if textures_processed.insert(tex.addr) {
                            if tex.dim.is_none() {
                                panic!("failed to resolve dimensions for {}", sym.resolve_seg_ptr(tex.addr));
                            }
                            if tex.format == IM_FMT_CI && tex.palette.is_none() {
                                panic!("no palette for {}", sym.resolve_seg_ptr(tex.addr));
                            }
            
                            textures.push(tex);
                        }
                    }

                    let palette = if let Some(tex) = palette.take() {
                        if *f == IM_FMT_CI {
                            Some(tex.addr)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let tex_addr = seg_state.addr_to_seg_ptr(*i);
                    cur_tex = Some(TextureData {
                        addr: tex_addr,
                        dim: tex_dim_hack.get(&tex_addr).copied(),
                        format: *f,
                        size: *s,
                        palette,
                    })
                }
                Gfx::SetTile(f, s, ..) => if let Some(tex) = &mut cur_tex {
                    tex.format = *f;
                    tex.size = match (*f, *s) {
                        // I16 and CI16 don't exist! I have no idea why Serene Fusion tries to use them
                        (IM_FMT_I, IM_SIZ_16B) => IM_SIZ_8B,
                        (IM_FMT_CI, IM_SIZ_16B) => IM_SIZ_4B,
                        _ => *s,
                    };
                }
                Gfx::LoadTile(_, _, _, lrs, lrt) | Gfx::SetTileSize(_, _, _, lrs, lrt) => {
                    if let Some(tex) = &mut cur_tex {
                        tex.dim = Some(((lrs >> 2) + 1, (lrt >> 2) + 1));
                    }
                }
                Gfx::LoadBlock(_, _, _, lrs, dxt) => {
                    if let Some(tex) = &mut cur_tex {
                        tex.dim = {
                            let width = dxt.reverse_bits() >> tex.size;
                            Some((width, (lrs + 1) / width))
                        };
                    }
                }
                Gfx::LoadTLUT(_, _) => {
                    if let Some(tex) = cur_tex.take() {
                        palette = Some(tex);
                    }
                }
                _ => {}
            }
        }
        if let Some(tex) = cur_tex {
            if textures_processed.insert(tex.addr) {
                if tex.dim.is_none() {
                    panic!("failed to resolve dimensions for {}", sym.resolve_seg_ptr(tex.addr));
                }
                if tex.format == IM_FMT_CI && tex.palette.is_none() {
                    panic!("no palette for {}", sym.resolve_seg_ptr(tex.addr));
                }

                textures.push(tex);
            }
        }

        nodes.push(gfx::to_c(&parsed, &sym.resolve_seg_ptr(addr), &seg_state, &sym)?);
    }

    println!("vertices...");
    for (addr, len) in vertex_buffers.into_vec() {
        let mut buf = BufWriter::new(Vec::new());
        writeln!(buf, "Vtx {}[] = {{", sym.resolve_seg_ptr(addr))?;

        let reader = &mut seg_data.get_reader_from_seg_ptr(addr)?;

        let read_u8 = |reader: &mut Cursor<&[u8]>| reader.read_u8();
        let read_u16 = |reader: &mut Cursor<&[u8]>| reader.read_u16::<BigEndian>();
        let read_i16 = |reader: &mut Cursor<&[u8]>| reader.read_i16::<BigEndian>();
        for _ in 0..len {
            // This is very fun to read
            writeln!(buf, 
                "    {{{{{{{}, {}, {}}}, {}, {{{}, {}}}, {{{:#04X}, {:#04X}, {:#04X}, {:#04X}}}}}}},",
                read_i16(reader)?, read_i16(reader)?, read_i16(reader)?,
                read_u16(reader)?,
                read_i16(reader)?, read_i16(reader)?,
                read_u8(reader)?, read_u8(reader)?, read_u8(reader)?, read_u8(reader)?,
            )?;
        }

        writeln!(buf, "}};")?;
        nodes.push(String::from_utf8(buf.into_inner()?)?);
    }

    println!("lights...");
    for (addr, _) in lights.into_vec() {
        let mut buf = BufWriter::new(Vec::new());
        writeln!(buf, "Light_t {} = {{", sym.resolve_seg_ptr(addr))?;

        let reader = &mut seg_data.get_reader_from_seg_ptr(addr)?;
        let read_u8 = |reader: &mut Cursor<&[u8]>| reader.read_u8();
        let read_i8 = |reader: &mut Cursor<&[u8]>| reader.read_i8();
        writeln!(buf, 
            "    {{{}, {}, {}}}, {}, {{{}, {}, {}}}, {}, {{{}, {}, {}}}, {}",
            read_u8(reader)?, read_u8(reader)?, read_u8(reader)?, read_u8(reader)?,
            read_u8(reader)?, read_u8(reader)?, read_u8(reader)?, read_u8(reader)?,
            read_i8(reader)?, read_i8(reader)?, read_i8(reader)?, read_u8(reader)?,
        )?;
        
        writeln!(buf, "}};")?;
        nodes.push(String::from_utf8(buf.into_inner()?)?);
    }

    // This has to be in a separate file because DynOS only parses rooms in room.inc.c files
    println!("rooms...");
    let room_nodes = rooms.into_vec().into_iter().map(|(addr, terrain_addr)| {
        let mut buf = BufWriter::new(Vec::new());

        let Some(tri_count) = surface_counts.get(&terrain_addr) else {
            panic!("no surface count for {}", sym.resolve_seg_ptr(terrain_addr));
        };
        let reader = &mut seg_data.get_reader_from_seg_ptr(addr).unwrap();

        let mut data = vec![0; *tri_count];
        reader.read_exact(&mut data).unwrap();

        write!(buf, "u8 {}[] = {{", sym.resolve_seg_ptr(addr)).unwrap();
        for x in data {
            write!(buf, "{},", x).unwrap();
        }
        writeln!(buf, "}};").unwrap();

        String::from_utf8(buf.into_inner().unwrap()).unwrap()
    }).collect::<Vec<_>>();

    println!("textures...");
    for tex in textures {
        let name = sym.resolve_seg_ptr(tex.addr);
        let dim = tex.dim.unwrap();
        let rgba = tex.as_rgba32(&seg_data)?;

        let file = BufWriter::new(File::create(format!("{BASE_PATH}\\map_tex\\{name}.png"))?);
        let mut encoder = png::Encoder::new(file, dim.0 as u32, dim.1 as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgba)?;

        nodes.push(format!("Texture {name} = \"../map_tex/{name}\";\n"));
        if let Some(palette) = tex.palette {
            // Make DynOS stop complaining that it can't find palettes despite everything being converted already
            let palette_name = sym.resolve_seg_ptr(palette);
            nodes.push(format!("Texture {palette_name} = \"../map_tex/dummy\";\n"));
        }
    };

    println!("writing files...");
    let mut script_file = BufWriter::new(File::create("D:\\Github\\sm64coopdx\\build\\us_pc\\mods\\serene-fusion\\levels\\asdf\\script.c")?);
    for x in nodes.iter().rev() {
        script_file.write_all(x.as_bytes())?;
    }
    let mut rooms_file = BufWriter::new(File::create("D:\\Github\\sm64coopdx\\build\\us_pc\\mods\\serene-fusion\\levels\\asdf\\room.inc.c")?);
    for x in room_nodes.iter().rev() {
        rooms_file.write_all(x.as_bytes())?;
    }

    println!("done!");
    Ok(())
}
