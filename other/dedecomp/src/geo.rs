use std::io::{BufWriter, Cursor, Write};

use byteorder::{BigEndian, ReadBytesExt};

use crate::common::{fix_layer, SegmentData, SegmentState, SegmentedPointer, SymbolState};

#[derive(Debug)]
pub enum GeoLayout {
    /* 0x00 */ BranchAndLink(u32),
    /* 0x01 */ End,
    /* 0x02 */ Branch(u8, u32),
    /* 0x03 */ Return,
    /* 0x04 */ OpenNode,
    /* 0x05 */ CloseNode,
    /* 0x08 */ ScreenArea(i16, i16, i16, i16, i16),
    /* 0x09 */ Ortho(i16),
    /* 0x0A */ Frustrum(i16, u16, u16, Option<u32>),
    /* 0x0B */ NodeStart,
    /* 0x0C */ ZBuffer(u8),
    /* 0x0D */ RenderRange(i16, i16),
    /* 0x0E */ SwitchCase(i16, u32),
    /* 0x0F */ Camera(i16, i16, i16, i16, i16, i16, i16, u32),
    /* 0x10 */ TranslateRotate(u8, i16, i16, i16, i16, i16, i16, u32),
    /* 0x11 */ Translate(u8, i16, i16, i16, u32),
    /* 0x12 */ Rotate(u8, i16, i16, i16, u32),
    /* 0x13 */ AnimatedPart(u8, i16, i16, i16, u32),
    /* 0x14 */ Billboard(u8, i16, i16, i16),
    /* 0x15 */ DisplayList(u8, u32),
    /* 0x16 */ Shadow(i16, i16, i16),
    /* 0x17 */ RenderObj,
    /* 0x18 */ Generated(i16, u32),
    /* 0x19 */ Background(i16, u32),
    /* 0x1C */ HeldObject(u8, i16, i16, i16, u32),
    /* 0x1D */ Scale(u8, u32, u32),
    /* 0x20 */ CullingRadius(i16),
}

impl GeoLayout {
    fn to_c(&self, seg: &SegmentState, sym: &SymbolState) -> String {
        let resolve = |x: &u32| sym.resolve_raw_ptr(seg, *x);

        match self {
            GeoLayout::BranchAndLink(target) => format!("GEO_BRANCH_AND_LINK({})", resolve(target)),
            GeoLayout::End => "GEO_END()".to_string(),
            GeoLayout::Branch(branch_type, target) => {
                format!("GEO_BRANCH({branch_type:#X}, {})", resolve(target))
            }
            GeoLayout::Return => "GEO_RETURN()".to_string(),
            GeoLayout::OpenNode => "GEO_OPEN_NODE()".to_string(),
            GeoLayout::CloseNode => "GEO_CLOSE_NODE()".to_string(),
            GeoLayout::ScreenArea(entries, x, y, width, height) => {
                format!("GEO_NODE_SCREEN_AREA({entries}, {x}, {y}, {width}, {height})")
            }
            GeoLayout::Ortho(scale) => format!("GEO_NODE_ORTHO({scale})"),
            GeoLayout::Frustrum(fov, near, far, func) => {
                if let Some(func) = func {
                    format!(
                        "GEO_CAMERA_FRUSTUM_WITH_FUNC({fov}, {near}, {far}, {})",
                        resolve(func)
                    )
                } else {
                    format!("GEO_CAMERA_FRUSTUM({fov}, {near}, {far})")
                }
            }
            GeoLayout::NodeStart => "GEO_NODE_START()".to_string(),
            GeoLayout::ZBuffer(enable) => format!("GEO_ZBUFFER({enable})"),
            GeoLayout::RenderRange(min, max) => format!("GEO_RENDER_RANGE({min}, {max})"),
            GeoLayout::SwitchCase(count, func) => {
                format!("GEO_SWITCH_CASE({count}, {})", resolve(func))
            }
            GeoLayout::Camera(camera_type, x, y, z, fx, fy, fz, func) => format!(
                "GEO_CAMERA({camera_type}, {x}, {y}, {z}, {fx}, {fy}, {fz}, {})",
                resolve(func)
            ),
            GeoLayout::TranslateRotate(params, tx, ty, tz, rx, ry, rz, display_list) => {
                let layer = params & 0xF;
                if (params & 0x80) != 0 {
                    match params & 0x30 {
                        0x00 => format!("GEO_TRANSLATE_ROTATE_WITH_DL({layer}, {tx}, {ty}, {tz}, {rx}, {ry}, {rz}, {})", resolve(display_list)),
                        0x10 => format!("GEO_TRANSLATE_WITH_DL({layer}, {tx}, {ty}, {tz}, {})", resolve(display_list)),
                        0x20 => format!("GEO_ROTATE_WITH_DL({layer}, {rx}, {ry}, {rz}, {})", resolve(display_list)),
                        0x30 => format!("GEO_ROTATE_Y_WITH_DL({layer}, {ry}, {})", resolve(display_list)),
                        _ => unreachable!()
                    }
                } else {
                    match params & 0x30 {
                        0x00 => format!(
                            "GEO_TRANSLATE_ROTATE({layer}, {tx}, {ty}, {tz}, {rx}, {ry}, {rz})"
                        ),
                        0x10 => format!("GEO_TRANSLATE({layer}, {tx}, {ty}, {tz})"),
                        0x20 => format!("GEO_ROTATE({layer}, {rx}, {ry}, {rz})"),
                        0x30 => format!("GEO_ROTATE_Y({layer}, {ry})"),
                        _ => unreachable!(),
                    }
                }
            }
            GeoLayout::Translate(params, x, y, z, display_list) => {
                let layer = params & 0xF;
                if (params & 0x80) != 0 {
                    format!(
                        "GEO_TRANSLATE_NODE_WITH_DL({layer}, {x}, {y}, {z}, {})",
                        resolve(display_list)
                    )
                } else {
                    format!("GEO_TRANSLATE_NODE({layer}, {x}, {y}, {z})")
                }
            }
            GeoLayout::Rotate(params, x, y, z, display_list) => {
                let layer = params & 0xF;
                if (params & 0x80) != 0 {
                    format!(
                        "GEO_ROTATE_WITH_DL({layer}, {x}, {y}, {z}, {})",
                        resolve(display_list)
                    )
                } else {
                    format!("GEO_ROTATE({layer}, {x}, {y}, {z})")
                }
            }
            GeoLayout::AnimatedPart(layer, x, y, z, display_list) => format!(
                "GEO_ANIMATED_PART({layer}, {x}, {y}, {z}, {})",
                resolve(display_list)
            ),
            GeoLayout::Billboard(layer, x, y, z) => {
                format!("GEO_BILLBOARD_WITH_PARAMS({layer}, {x}, {y}, {z})")
            }
            GeoLayout::DisplayList(layer, display_list) => {
                format!("GEO_DISPLAY_LIST({layer}, {})", resolve(display_list))
            }
            GeoLayout::RenderObj => "GEO_RENDER_OBJ()".to_string(),
            GeoLayout::Scale(params, scale, display_list) => {
                let layer = params & 0xF;
                if (params & 0x80) != 0 {
                    format!(
                        "GEO_SCALE_WITH_DL({layer}, {scale}, {})",
                        resolve(display_list)
                    )
                } else {
                    format!("GEO_SCALE({layer}, {scale})")
                }
            }
            GeoLayout::Shadow(shadow_type, solidity, scale) => {
                format!("GEO_SHADOW({shadow_type}, {solidity}, {scale})")
            }
            GeoLayout::Generated(param, func) => format!("GEO_ASM({param:#X}, {})", resolve(func)),
            GeoLayout::Background(id, func) => format!("GEO_BACKGROUND({id}, {})", resolve(func)),
            GeoLayout::HeldObject(param, x, y, z, func) => format!(
                "GEO_HELD_OBJECT({param:#X}, {x}, {y}, {z}, {})",
                resolve(func)
            ),
            GeoLayout::CullingRadius(radius) => format!("GEO_CULLING_RADIUS({radius})"),
        }
    }
}

pub fn parse_script(
    seg_data: &SegmentData,
    addr: SegmentedPointer,
) -> Result<Vec<GeoLayout>, std::io::Error> {
    let mut ret = vec![];
    let reader = &mut seg_data.get_reader_from_seg_ptr(addr)?;

    let read_u8 = |reader: &mut Cursor<&[u8]>| reader.read_u8();
    let read_u16 = |reader: &mut Cursor<&[u8]>| reader.read_u16::<BigEndian>();
    let read_i16 = |reader: &mut Cursor<&[u8]>| reader.read_i16::<BigEndian>();
    let read_u32 = |reader: &mut Cursor<&[u8]>| reader.read_u32::<BigEndian>();
    let mut stop = false;
    while !stop {
        let cmd = read_u8(reader)?;
        //writeln!(buf,"// {cmd:#X}");

        let decoded = match cmd {
            0x00 => {
                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                let target = read_u32(reader)?;
                GeoLayout::BranchAndLink(target)
            }
            0x01 => {
                stop = true;

                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                GeoLayout::End
            }
            0x02 => {
                let branch_type = read_u8(reader)?;
                assert_eq!(read_u16(reader)?, 0);
                let target = read_u32(reader)?;
                GeoLayout::Branch(branch_type, target)
            }
            0x03 => {
                stop = true;

                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                GeoLayout::Return
            }
            0x04 => {
                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                GeoLayout::OpenNode
            }
            0x05 => {
                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                GeoLayout::CloseNode
            }
            0x08 => {
                assert_eq!(read_u8(reader)?, 0);
                let entries = read_i16(reader)?;
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let width = read_i16(reader)?;
                let height = read_i16(reader)?;
                GeoLayout::ScreenArea(entries, x, y, width, height)
            }
            0x09 => {
                assert_eq!(read_u8(reader)?, 0);
                let scale = read_i16(reader)?;
                GeoLayout::Ortho(scale)
            }
            0x0A => {
                let has_func = read_u8(reader)?;
                let fov = read_i16(reader)?;
                let near = read_u16(reader)?;
                let far = read_u16(reader)?;
                let func = if has_func != 0 {
                    Some(read_u32(reader)?)
                } else {
                    None
                };
                GeoLayout::Frustrum(fov, near, far, func)
            }
            0x0B => {
                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                GeoLayout::NodeStart
            }
            0x0C => {
                let enable = read_u8(reader)?;
                assert_eq!(read_u16(reader)?, 0);
                GeoLayout::ZBuffer(enable)
            }
            0x0D => {
                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                let min = read_i16(reader)?;
                let max = read_i16(reader)?;
                GeoLayout::RenderRange(min, max)
            }
            0x0E => {
                assert_eq!(read_u8(reader)?, 0);
                let count = read_i16(reader)?;
                let func = read_u32(reader)?;
                GeoLayout::SwitchCase(count, func)
            }
            0x0F => {
                assert_eq!(read_u8(reader)?, 0);
                let camera_type = read_i16(reader)?;
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                let fx = read_i16(reader)?;
                let fy = read_i16(reader)?;
                let fz = read_i16(reader)?;
                let func = read_u32(reader)?;
                GeoLayout::Camera(camera_type, x, y, z, fx, fy, fz, func)
            }
            0x10 => {
                let params = read_u8(reader)?;
                assert_eq!(read_u16(reader)?, 0);
                let (tx, ty, tz, rx, ry, rz) = match params & 0x30 {
                    0x00 => (
                        read_i16(reader)?,
                        read_i16(reader)?,
                        read_i16(reader)?,
                        read_i16(reader)?,
                        read_i16(reader)?,
                        read_i16(reader)?,
                    ),
                    0x10 => (
                        read_i16(reader)?,
                        read_i16(reader)?,
                        read_i16(reader)?,
                        0,
                        0,
                        0,
                    ),
                    0x20 => (
                        0,
                        0,
                        0,
                        read_i16(reader)?,
                        read_i16(reader)?,
                        read_i16(reader)?,
                    ),
                    0x30 => (0, 0, 0, 0, read_i16(reader)?, 0),
                    _ => unreachable!(),
                };
                let display_list = if (params & 0x80) != 0 {
                    read_u32(reader)?
                } else {
                    0
                };
                GeoLayout::TranslateRotate(params, tx, ty, tz, rx, ry, rz, display_list)
            }
            0x11 => {
                let layer = fix_layer(read_u8(reader)?);
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                let display_list = if (layer & 0x80) != 0 {
                    read_u32(reader)?
                } else {
                    0
                };
                GeoLayout::Translate(layer, x, y, z, display_list)
            }
            0x12 => {
                let layer = fix_layer(read_u8(reader)?);
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                let display_list = if (layer & 0x80) != 0 {
                    read_u32(reader)?
                } else {
                    0
                };
                GeoLayout::Rotate(layer, x, y, z, display_list)
            }
            0x13 => {
                let layer = fix_layer(read_u8(reader)?);
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                let display_list = read_u32(reader)?;
                GeoLayout::AnimatedPart(layer, x, y, z, display_list)
            }
            0x14 => {
                let layer = fix_layer(read_u8(reader)?);
                assert!((layer & 0x80) == 0);
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                GeoLayout::Billboard(layer, x, y, z)
            }
            0x15 => {
                let layer = fix_layer(read_u8(reader)?);
                assert_eq!(read_u16(reader)?, 0);
                let display_list = read_u32(reader)?;
                GeoLayout::DisplayList(layer, display_list)
            }
            0x16 => {
                assert_eq!(read_u8(reader)?, 0);
                let shadow_type = read_i16(reader)?;
                let solidity = read_i16(reader)?;
                let scale = read_i16(reader)?;
                GeoLayout::Shadow(shadow_type, solidity, scale)
            }
            0x17 => {
                assert_eq!(read_u8(reader)?, 0);
                assert_eq!(read_u16(reader)?, 0);
                GeoLayout::RenderObj
            }
            0x18 => {
                assert_eq!(read_u8(reader)?, 0);
                let param = read_i16(reader)?;
                let func = read_u32(reader)?;
                GeoLayout::Generated(param, func)
            }
            0x19 => {
                assert_eq!(read_u8(reader)?, 0);
                let id = read_i16(reader)?;
                let func = read_u32(reader)?;
                GeoLayout::Background(id, func)
            }
            0x1C => {
                let param = read_u8(reader)?;
                let x = read_i16(reader)?;
                let y = read_i16(reader)?;
                let z = read_i16(reader)?;
                let func = read_u32(reader)?;
                GeoLayout::HeldObject(param, x, y, z, func)
            }
            0x1D => {
                let layer = fix_layer(read_u8(reader)?);
                assert_eq!(read_u16(reader)?, 0);
                let scale = read_u32(reader)?;
                let display_list = if (layer & 0x80) != 0 {
                    read_u32(reader)?
                } else {
                    0
                };
                GeoLayout::Scale(layer, scale, display_list)
            }
            0x20 => {
                assert_eq!(read_u8(reader)?, 0);
                let radius = read_i16(reader)?;
                GeoLayout::CullingRadius(radius)
            }
            x => unimplemented!("Unhandled command {x:#X}"),
        };
        ret.push(decoded);
    }

    Ok(ret)
}

pub fn to_c(
    script: &[GeoLayout],
    name: &str,
    seg_common: &SegmentState,
    sym: &SymbolState,
) -> Result<String, std::io::Error> {
    let mut buf = BufWriter::new(Vec::new());

    let mut indent = 4;
    writeln!(buf, "GeoLayout {name}[] = {{")?;
    for instr in script {
        if matches!(instr, GeoLayout::CloseNode) {
            indent -= 4;
        }
        writeln!(buf, "{:indent$}{},", "", instr.to_c(seg_common, sym))?;
        if matches!(instr, GeoLayout::OpenNode) {
            indent += 4;
        }
    }
    writeln!(buf, "}};")?;

    Ok(String::from_utf8(buf.into_inner()?).unwrap())
}
