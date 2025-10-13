use std::io::{BufWriter, Cursor, Write};

use byteorder::{BigEndian, ReadBytesExt};

use crate::common::{
    fix_layer, SegmentBase, SegmentData, SegmentState, SegmentedPointer, SymbolState,
};

#[derive(Debug)]
pub enum LevelScript {
    /* 0x00 */ Execute(u16, u32, u32, u32, u32, u32),
    /* 0x01 */ ExitAndExecute(u16, u32, u32, u32),
    /* 0x02 */ Exit,
    /* 0x03 */ Sleep(u16),
    /* 0x04 */ SleepBeforeExit(u16),
    /* 0x05 */ Jump(u32),
    /* 0x06 */ JumpLink(u32),
    /* 0x07 */ Return,
    /* 0x08 */ JumpLinkPushArg(u16),
    /* 0x09 */ JumpNTimes,
    /* 0x0A */ LoopBegin,
    /* 0x0B */ LoopUntil(u8, u32),
    /* 0x0C */ JumpIf(u8, u32, u32),
    /* 0x11 */ Call(u16, u32),
    /* 0x12 */ CallLoop(u16, u32),
    /* 0x13 */ SetReg(u16),
    /* 0x14 */ PushPool,
    /* 0x15 */ PopPool,
    /* 0x16 */ FixedLoad(u32, u32, u32),
    /* 0x17 */ LoadRaw(u16, u32, u32, u32, u32),
    /* 0x18 */ LoadMIO0(u16, u32, u32),
    /* 0x1A */ LoadMIO0Texture(u16, u32, u32),
    /* 0x1B */ InitLevel,
    /* 0x1C */ ClearLevel,
    /* 0x1D */ AllocLevelPool,
    /* 0x1E */ FreeLevelPool,
    /* 0x1F */ BeginArea(u8, u32),
    /* 0x20 */ EndArea,
    /* 0x21 */ LoadModelFromDL(u16, u32, u16),
    /* 0x22 */ LoadModelFromGeo(u16, u32),
    /* 0x24 */ PlaceObject(u8, i16, i16, i16, i16, i16, i16, u32, u32, u32),
    /* 0x25 */ Mario(u8, u32, u32),
    /* 0x26 */ WarpNode(u8, u8, u8, u8, u8),
    /* 0x27 */ PaintingWarpNode(u8, u8, u8, u8, u8),
    /* 0x28 */ InstantWarp(u8, u8, u32, u32, u32),
    /* 0x29 */ LoadArea(u8),
    /* 0x2A */ UnloadArea(u8),
    /* 0x2B */ MarioPos(u8, i16, i16, i16, i16),
    /* 0x2D */ UpdateObjects,
    /* 0x2E */ Terrain(u32),
    /* 0x2F */ Rooms(u32),
    /* 0x30 */ ShowDialog(u8, u8),
    /* 0x31 */ TerrainType(u16),
    /* 0x33 */ Transition(u8, u8, u8, u8, u8),
    /* 0x34 */ Blackout(u8),
    /* 0x36 */ SetBackgroundMusic(u16, u16),
    /* 0x37 */ SetMenuMusic(u16),
    /* 0x38 */ StopMusic(u16),
    /* 0x39 */ MacroObjects(u32),
    /* 0x3B */ Whirlpool(u8, u8, u16, u16, u16, u16),
    /* 0x3C */ GetOrSet(u8, u8),
}

impl LevelScript {
    fn to_c(&self, seg_state: &SegmentState, sym: &SymbolState) -> String {
        let resolve = |x: &u32| sym.resolve_raw_ptr(seg_state, *x);

        match self {
            LevelScript::Execute(seg, script, script_end, entry, _bss_start, _bss_end) => {
                let mut temp = seg_state.clone();
                temp.set_seg_base(*seg as usize, SegmentBase::Raw(*script));
                /*
                if *bss_start == *bss_end {
                    format!("EXECUTE({seg:#X}, {script:#X}, {script_end:#X}, {})", sym.resolve_seg_ptr(&temp, *entry))
                } else {
                    format!("EXECUTE_WITH_CODE({seg:#X}, {script:#X}, {script_end:#X}, {}, {bss_start:#X}, {bss_end:#X})", sym.resolve_seg_ptr(&temp, *entry))
                }
                */
                format!(
                    "EXECUTE({seg:#X}, {script:#X}, {script_end:#X}, {})",
                    sym.resolve_raw_ptr(&temp, *entry)
                )
            }
            LevelScript::ExitAndExecute(seg, script, script_end, entry) => {
                let mut temp = seg_state.clone();
                temp.set_seg_base(*seg as usize, SegmentBase::Raw(*script));
                format!(
                    "EXIT_AND_EXECUTE({seg:#X}, {script:#X}, {script_end:#X}, {})",
                    sym.resolve_raw_ptr(&temp, *entry)
                )
            }
            LevelScript::Exit => "EXIT()".to_string(),
            LevelScript::Sleep(frames) => format!("SLEEP({frames})"),
            LevelScript::SleepBeforeExit(frames) => format!("SLEEP_BEFORE_EXIT({frames})"),
            LevelScript::Jump(target) => format!("JUMP({})", resolve(target)),
            LevelScript::JumpLink(target) => format!("JUMP_LINK({})", resolve(target)),
            LevelScript::Return => "RETURN()".to_string(),
            LevelScript::JumpLinkPushArg(arg) => format!("JUMP_LINK_PUSH_ARG({arg:#X})"),
            LevelScript::JumpNTimes => "JUMP_N_TIMES()".to_string(),
            LevelScript::LoopBegin => "LOOP_BEGIN()".to_string(),
            LevelScript::LoopUntil(op, arg) => format!("LOOP_UNTIL({op:#X}, {arg:#X})"),
            LevelScript::JumpIf(op, arg, target) => {
                format!("JUMP_IF({op:#X}, {arg:#X}, {})", resolve(target))
            }
            LevelScript::Call(arg, func) => format!("CALL({arg:#X}, {})", resolve(func)),
            LevelScript::CallLoop(arg, func) => format!("CALL_LOOP({arg:#X}, {})", resolve(func)),
            LevelScript::SetReg(value) => format!("SET_REG({value:#X})"),
            LevelScript::PushPool => "PUSH_POOL()".to_string(),
            LevelScript::PopPool => "POP_POOL()".to_string(),
            LevelScript::FixedLoad(load_addr, rom_start, rom_end) => {
                format!("// FIXED_LOAD({load_addr:#X}, {rom_start:#X}, {rom_end:#X})")
            }
            LevelScript::LoadRaw(seg, rom_start, rom_end, _bss_start, _bss_end) => {
                /*
                if *bss_start == *bss_end {
                    format!("//LOAD_RAW({seg:#X}, {rom_start:#X}, {rom_end:#X})")
                } else {
                    format!("//LOAD_RAW_WITH_CODE({seg:#X}, {rom_start:#X}, {rom_end:#X}, {bss_start:#X}, {bss_end:#X})")
                }
                */
                format!("// LOAD_RAW({seg:#X}, {rom_start:#X}, {rom_end:#X})")
            }
            LevelScript::LoadMIO0(seg, rom_start, rom_end) => {
                format!("// LOAD_MIO0({seg:#X}, {rom_start:#X}, {rom_end:#X})")
            }
            LevelScript::LoadMIO0Texture(seg, rom_start, rom_end) => {
                format!("// LOAD_MIO0_TEXTURE({seg:#X}, {rom_start:#X}, {rom_end:#X})")
            }
            LevelScript::InitLevel => "INIT_LEVEL()".to_string(),
            LevelScript::ClearLevel => "CLEAR_LEVEL()".to_string(),
            LevelScript::AllocLevelPool => "ALLOC_LEVEL_POOL()".to_string(),
            LevelScript::FreeLevelPool => "FREE_LEVEL_POOL()".to_string(),
            LevelScript::BeginArea(level, geo) => format!("AREA({level:#X}, {})", resolve(geo)),
            LevelScript::EndArea => "END_AREA()".to_string(),
            LevelScript::LoadModelFromDL(model, dl, layer) => format!(
                "//LOAD_MODEL_FROM_DL({model:#X}, {}, {layer:#X})",
                resolve(dl)
            ),
            LevelScript::LoadModelFromGeo(model, geo) => {
                format!("LOAD_MODEL_FROM_GEO({model:#X}, {})", resolve(geo))
            }
            LevelScript::PlaceObject(
                acts,
                x,
                y,
                z,
                angle_x,
                angle_y,
                angle_z,
                beh_param,
                beh,
                model,
            ) => {
                if *acts == 0x1F {
                    format!("//OBJECT({model:#X}, {x}, {y}, {z}, {angle_x}, {angle_y}, {angle_z}, {beh_param:#X}, {})", resolve(beh))
                } else {
                    format!("//OBJECT_WITH_ACTS({model:#X}, {x}, {y}, {z}, {angle_x}, {angle_y}, {angle_z}, {beh_param:#X}, {}, {acts:#X})", resolve(beh))
                }
            }
            LevelScript::Mario(model, bhv_arg, bhv) => {
                format!("MARIO({model:#X}, {bhv_arg:#X}, {})", resolve(bhv))
            }
            LevelScript::WarpNode(id, level, area, node, flags) => {
                format!("WARP_NODE({id:#X}, {level:#X}, {area:#X}, {node:#X}, {flags:#X})")
            }
            LevelScript::PaintingWarpNode(id, level, area, node, flags) => {
                format!("WARP_NODE({id:#X}, {level:#X}, {area:#X}, {node:#X}, {flags:#X})")
            }
            LevelScript::InstantWarp(id, area, x, y, z) => {
                format!("INSTANT_WARP({id:#X}, {area:#X}, {x:#X}, {y:#X}, {z:#X})")
            }
            LevelScript::LoadArea(area) => format!("LOAD_AREA({area:#X})"),
            LevelScript::UnloadArea(area) => format!("CMD2A({area:#X})"),
            LevelScript::MarioPos(area, yaw, x, y, z) => {
                format!("MARIO_POS({area:#X}, {yaw}, {x}, {y}, {z})")
            }
            LevelScript::UpdateObjects => "CMD2D()".to_string(),
            LevelScript::Terrain(terrain_data) => format!("TERRAIN({})", resolve(terrain_data)),
            LevelScript::Rooms(surface_rooms) => format!("ROOMS({})", resolve(surface_rooms)),
            LevelScript::ShowDialog(index, id) => format!("SHOW_DIALOG({index:#X}, {id:#X})"),
            LevelScript::TerrainType(terrain_type) => format!("TERRAIN_TYPE({terrain_type:#X})"),
            LevelScript::Transition(trans_type, time, r, g, b) => {
                format!("TRANSITION({trans_type:#X}, {time}, {r:#X}, {g:#X}, {b:#X})")
            }
            LevelScript::Blackout(active) => format!("BLACKOUT({active:#X})"),
            LevelScript::SetBackgroundMusic(preset, seq) => {
                format!("SET_BACKGROUND_MUSIC({preset:#X}, {seq:#X})")
            }
            LevelScript::SetMenuMusic(seq) => format!("SET_MENU_MUSIC({seq:#X})"),
            LevelScript::StopMusic(fade_out_time) => format!("STOP_MUSIC({fade_out_time:#X})"),
            LevelScript::MacroObjects(obj_list) => {
                format!("//MACRO_OBJECTS({})", resolve(obj_list))
            }
            LevelScript::Whirlpool(index, acts, x, y, z, strength) => {
                format!("WHIRLPOOL({index:#X}, {acts:#X}, {x:#X}, {y:#X}, {z:#X}, {strength:#X})")
            }
            LevelScript::GetOrSet(op, var) => format!("GET_OR_SET({op:#X}, {var:#X})"),
        }
    }
}

pub fn parse_script(
    seg_data: &SegmentData,
    addr: SegmentedPointer,
) -> Result<Vec<LevelScript>, std::io::Error> {
    let mut ret = vec![];
    let reader = &mut seg_data.get_reader_from_seg_ptr(addr)?;

    let read_u8 = |reader: &mut Cursor<&[u8]>| reader.read_u8();
    let read_u16 = |reader: &mut Cursor<&[u8]>| reader.read_u16::<BigEndian>();
    let read_i16 = |reader: &mut Cursor<&[u8]>| reader.read_i16::<BigEndian>();
    let read_u32 = |reader: &mut Cursor<&[u8]>| reader.read_u32::<BigEndian>();
    let mut stop = false;
    while !stop {
        let cmd = read_u8(reader)?;
        let len = read_u8(reader)?;

        //writeln!(buf, "// {len}");
        let decoded = match cmd {
            0x00 => {
                let seg = read_u16(reader)?;
                let script = read_u32(reader)?;
                let script_end = read_u32(reader)?;
                let entry = read_u32(reader)?;
                let bss_start = read_u32(reader)?;
                let bss_end = read_u32(reader)?;
                LevelScript::Execute(seg, script, script_end, entry, bss_start, bss_end)
            }
            0x01 => {
                stop = true;

                let seg = read_u16(reader)?;
                let script = read_u32(reader)?;
                let script_end = read_u32(reader)?;
                let entry = read_u32(reader)?;
                LevelScript::ExitAndExecute(seg, script, script_end, entry)
            }
            0x02 => {
                stop = true;

                assert_eq!(read_u16(reader)?, 0);
                LevelScript::Exit
            }
            0x03 => {
                let frames = read_u16(reader)?;
                LevelScript::Sleep(frames)
            }
            0x04 => {
                let frames = read_u16(reader)?;
                LevelScript::SleepBeforeExit(frames)
            }
            0x05 => {
                stop = true;

                assert_eq!(read_u16(reader)?, 0);
                let target = read_u32(reader)?;
                LevelScript::Jump(target)
            }
            0x06 => {
                assert_eq!(read_u16(reader)?, 0);
                let target = read_u32(reader)?;
                LevelScript::JumpLink(target)
            }
            0x07 => {
                stop = true;

                assert_eq!(read_u16(reader)?, 0);
                LevelScript::Return
            }
            0x08 => {
                let arg = read_u16(reader)?;
                LevelScript::JumpLinkPushArg(arg)
            }
            0x09 => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::JumpNTimes
            }
            0x0A => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::LoopBegin
            }
            0x0B => {
                let op = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                let arg = read_u32(reader)?;
                LevelScript::LoopUntil(op, arg)
            }
            0x0C => {
                let op = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                let arg = read_u32(reader)?;
                let target = read_u32(reader)?;
                LevelScript::JumpIf(op, arg, target)
            }
            0x11 => {
                let arg = read_u16(reader)?;
                let func = read_u32(reader)?;
                LevelScript::Call(arg, func)
            }
            0x12 => {
                let arg = read_u16(reader)?;
                let func = read_u32(reader)?;
                LevelScript::CallLoop(arg, func)
            }
            0x13 => {
                let value = read_u16(reader)?;
                LevelScript::SetReg(value)
            }
            0x14 => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::PushPool
            }
            0x15 => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::PopPool
            }
            0x16 => {
                assert_eq!(read_u16(reader)?, 0);
                let load_addr = read_u32(reader)?;
                let rom_start = read_u32(reader)?;
                let rom_end = read_u32(reader)?;
                LevelScript::FixedLoad(load_addr, rom_start, rom_end)
            }
            0x17 => {
                let seg = read_u16(reader)?;
                let rom_start = read_u32(reader)?;
                let rom_end = read_u32(reader)?;
                let bss_start = read_u32(reader)?;
                let bss_end = read_u32(reader)?;
                LevelScript::LoadRaw(seg, rom_start, rom_end, bss_start, bss_end)
            }
            0x18 => {
                let seg = read_u16(reader)?;
                let rom_start = read_u32(reader)?;
                let rom_end = read_u32(reader)?;
                LevelScript::LoadMIO0(seg, rom_start, rom_end)
            }
            0x1A => {
                let seg = read_u16(reader)?;
                let rom_start = read_u32(reader)?;
                let rom_end = read_u32(reader)?;
                LevelScript::LoadMIO0Texture(seg, rom_start, rom_end)
            }
            0x1B => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::InitLevel
            }
            0x1C => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::ClearLevel
            }
            0x1D => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::AllocLevelPool
            }
            0x1E => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::FreeLevelPool
            }
            0x1F => {
                let level = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                let geo = read_u32(reader)?;
                LevelScript::BeginArea(level, geo)
            }
            0x20 => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::EndArea
            }
            0x21 => {
                assert_eq!(read_u16(reader)?, 0);
                let dl = read_u32(reader)?;
                let layer = fix_layer(read_u16(reader)?);
                let model = read_u16(reader)?;
                LevelScript::LoadModelFromDL(model, dl, layer)
            }
            0x22 => {
                let model = read_u16(reader)?;
                let geo = read_u32(reader)?;
                LevelScript::LoadModelFromGeo(model, geo)
            }
            0x24 => {
                let acts = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                let angle_x = read_i16(reader)?;
                let angle_y = read_i16(reader)?;
                let angle_z = read_i16(reader)?;
                let beh_param = read_u32(reader)?;
                let beh = read_u32(reader)?;
                let model = read_u32(reader)?;
                LevelScript::PlaceObject(
                    acts, x, y, z, angle_x, angle_y, angle_z, beh_param, beh, model,
                )
            }
            0x25 => {
                assert_eq!(read_u8(reader)?, 0);
                let model = read_u8(reader)?;
                let bhv_arg = read_u32(reader)?;
                let bhv = read_u32(reader)?;
                LevelScript::Mario(model, bhv_arg, bhv)
            }
            0x26 => {
                let id = read_u8(reader)?;
                let level = read_u8(reader)?;
                let area = read_u8(reader)?;
                let node = read_u8(reader)?;
                let flags = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                LevelScript::WarpNode(id, level, area, node, flags)
            }
            0x27 => {
                let id = read_u8(reader)?;
                let level = read_u8(reader)?;
                let area = read_u8(reader)?;
                let node = read_u8(reader)?;
                let flags = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                LevelScript::PaintingWarpNode(id, level, area, node, flags)
            }
            0x28 => {
                let id = read_u8(reader)?;
                let area = read_u8(reader)?;
                let x = read_u32(reader)?;
                let y = read_u32(reader)?;
                let z = read_u32(reader)?;
                LevelScript::InstantWarp(id, area, x, y, z)
            }
            0x29 => {
                let area = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                LevelScript::LoadArea(area)
            }
            0x2A => {
                let area = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                LevelScript::UnloadArea(area)
            }
            0x2B => {
                let area = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                let yaw = read_i16(reader)?;
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                LevelScript::MarioPos(area, yaw, x, y, z)
            }
            0x2D => {
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::UpdateObjects
            }
            0x2E => {
                assert_eq!(read_u16(reader)?, 0);
                let terrain_data = read_u32(reader)?;
                LevelScript::Terrain(terrain_data)
            }
            0x2F => {
                assert_eq!(read_u16(reader)?, 0);
                let surface_rooms = read_u32(reader)?;
                LevelScript::Rooms(surface_rooms)
            }
            0x30 => {
                let index = read_u8(reader)?;
                let dialog_id = read_u8(reader)?;
                LevelScript::ShowDialog(index, dialog_id)
            }
            0x31 => {
                let terrain_type = read_u16(reader)?;
                LevelScript::TerrainType(terrain_type)
            }
            0x33 => {
                let trans_type = read_u8(reader)?;
                let time = read_u8(reader)?;
                let r = read_u8(reader)?;
                let g = read_u8(reader)?;
                let b = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                LevelScript::Transition(trans_type, time, r, g, b)
            }
            0x34 => {
                let active = read_u8(reader)?;
                assert_eq!(read_u8(reader)?, 0);
                LevelScript::Blackout(active)
            }
            0x36 => {
                let preset = read_u16(reader)?;
                let seq = read_u16(reader)?;
                assert_eq!(read_u16(reader)?, 0);
                LevelScript::SetBackgroundMusic(preset, seq)
            }
            0x37 => {
                let seq = read_u16(reader)?;
                LevelScript::SetMenuMusic(seq)
            }
            0x38 => {
                let fade_out_time = read_u16(reader)?;
                LevelScript::StopMusic(fade_out_time)
            }
            0x39 => {
                assert_eq!(read_u16(reader)?, 0);
                let obj_list = read_u32(reader)?;
                LevelScript::MacroObjects(obj_list)
            }
            0x3B => {
                let index = read_u8(reader)?;
                let acts = read_u8(reader)?;
                let x = read_u16(reader)?;
                let y = read_u16(reader)?;
                let z = read_u16(reader)?;
                let strength = read_u16(reader)?;
                LevelScript::Whirlpool(index, acts, x, y, z, strength)
            }
            0x3C => {
                let op = read_u8(reader)?;
                let var = read_u8(reader)?;
                LevelScript::GetOrSet(op, var)
            }
            x => unimplemented!("Unhandled command {x:#X} with len {len:#X}"),
        };
        ret.push(decoded);
    }

    Ok(ret)
}

pub fn to_c(
    script: &[LevelScript],
    name: &str,
    seg_state: &mut SegmentState,
    sym: &SymbolState,
) -> Result<String, std::io::Error> {
    let mut buf = BufWriter::new(Vec::new());

    let mut indent = 4;
    writeln!(buf, "LevelScript {name}[] = {{")?;
    for instr in script {
        if matches!(instr, LevelScript::EndArea) {
            indent -= 4;
        }
        writeln!(buf, "{:indent$}{},", "", instr.to_c(seg_state, sym))?;
        match instr {
            LevelScript::LoadMIO0(seg, start, _end) => {
                seg_state.set_seg_base(*seg as usize, SegmentBase::MIO0(*start))
            }
            LevelScript::LoadRaw(seg, start, _end, _bss_start, _bss_end) => {
                seg_state.set_seg_base(*seg as usize, SegmentBase::Raw(*start))
            }
            LevelScript::BeginArea(_level, _geo) => indent += 4,
            _ => {}
        }
    }
    writeln!(buf, "}};")?;

    Ok(String::from_utf8(buf.into_inner()?).unwrap())
}
