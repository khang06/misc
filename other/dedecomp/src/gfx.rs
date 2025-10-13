use std::io::{BufWriter, Write};

use byteorder::{BigEndian, ReadBytesExt};

use crate::common::{SegmentData, SegmentState, SegmentedPointer, SymbolState};

// NOTE: This is targeting F3DZEX!!!

#[derive(Debug)]
pub enum Gfx {
    /* 0x01 */      Vertex(u32, u8, u8),
    /* 0x03 */      Cull(u16, u16),
    /* 0x05 */      Triangle(u8, u8, u8),
    /* 0x06 */      TwoTriangles(u8, u8, u8, u8, u8, u8),
    /* 0xD7 */      Texture(u16, u16, u8, u8, u8),
    /* 0xD9 */      GeometryMode(u32, u32),
    /* 0xDB */      MoveWord(u8, u16, u32),
    /* 0xDC */      MoveMem(u32, u16, u8, u16),
    /* 0xDE */      DisplayList(u32),
    /* 0xDE (2) */  BranchList(u32),
    /* 0xDF */      End,
    /* 0xE2-0xE3 */ SetOtherMode(u8, u8, u8, u32),
    /* 0xE6 */      LoadSync,
    /* 0xE7 */      PipeSync,
    /* 0xE8 */      TileSync,
    /* 0xEE */      SetPrimDepth(u16, u16),
    /* 0xF0 */      LoadTLUT(u8, u16),
    /* 0xF2 */      SetTileSize(u8, u16, u16, u16, u16),
    /* 0xF3 */      LoadBlock(u8, u16, u16, u16, u16),
    /* 0xF4 */      LoadTile(u8, u16, u16, u16, u16),
    /* 0xF5 */      SetTile(u8, u8, u16, u16, u8, u8, u8, u8, u8, u8, u8, u8),
    /* 0xF8 */      SetFogColor(u8, u8, u8, u8),
    /* 0xF9 */      SetBlendColor(u8, u8, u8, u8),
    /* 0xFA */      SetPrimColor(u8, u8, u8, u8, u8, u8),
    /* 0xFB */      SetEnvColor(u8, u8, u8, u8),
    /* 0xFC */      SetCombine(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8),
    /* 0xFD */      SetTextureImage(u8, u8, u16, u32),

    Unhandled(u64),
}

impl Gfx {
    fn to_c(&self, seg_state: &SegmentState, sym: &SymbolState) -> String {
        let resolve = |x: &u32| sym.resolve_raw_ptr(seg_state, *x);

        match self {
            Gfx::Vertex(v, n, v0) => format!("gsSPVertex({}, {n}, {v0})", resolve(v)),
            Gfx::Cull(vstart, vend) => format!("gsSPCullDisplayList({vstart}, {vend})"),
            Gfx::Triangle(v0, v1, v2) => format!("gsSP1Triangle({v0}, {v1}, {v2}, 0)"),
            Gfx::TwoTriangles(v00, v01, v02, v10, v11, v12) => format!("gsSP2Triangles({v00}, {v01}, {v02}, 0, {v10}, {v11}, {v12}, 0)"),
            Gfx::Texture(s, t, level, tile, on) => format!("gsSPTexture({s:#X}, {t:#X}, {level}, {tile}, {on})"),
            Gfx::GeometryMode(c, s) => format!("gsSPGeometryMode({c:#X}, {s:#X})"),
            Gfx::MoveWord(index, offset, data) => format!("gsMoveWd({index}, {offset}, {data:#X})"),
            Gfx::MoveMem(adrs, len, idx, ofs) => match idx {
                0xA => {
                    assert_eq!(*len, 16);
                    format!("gsSPLight({}, {})", resolve(adrs), (ofs - 24) / 24)
                }
                x => unimplemented!("Unhandled gsDma2p idx {x:#X}")
            }
            Gfx::DisplayList(dl) => format!("gsSPDisplayList({})", resolve(dl)),
            Gfx::BranchList(dl) => format!("gsSPBranchList({})", resolve(dl)),
            Gfx::End => "gsSPEndDisplayList()".to_string(),
            Gfx::SetOtherMode(cmd, sft, len, data) => format!("gsSPSetOtherMode({cmd:#X}, {sft}, {len}, {data:#X})"),
            Gfx::LoadSync => "gsDPLoadSync()".to_string(),
            Gfx::PipeSync => "gsDPPipeSync()".to_string(),
            Gfx::TileSync => "gsDPTileSync()".to_string(),
            Gfx::SetPrimDepth(z, dz) => format!("gDPSetPrimDepth({z}, {dz})"),
            Gfx::LoadTLUT(tile, count) => format!("gsDPLoadTLUTCmd({tile:#X}, {count})"),
            Gfx::SetTileSize(tile, uls, ult, lrs, lrt) => format!("gsDPSetTileSize({tile}, {uls}, {ult}, {lrs}, {lrt})"),
            Gfx::LoadBlock(tile, uls, ult, lrs, dxt) => format!("gsDPLoadBlock({tile}, {uls}, {ult}, {lrs}, {dxt})"),
            Gfx::LoadTile(tile, uls, ult, lrs, lrt) => format!("gsDPLoadTile({tile}, {uls}, {ult}, {lrs}, {lrt})"),
            Gfx::SetTile(fmt, siz, line, tmem, tile, palette, cmt, maskt, shiftt, cms, masks, shifts) =>
                format!("gsDPSetTile({fmt}, {siz}, {line}, {tmem}, {tile}, {palette}, {cmt}, {maskt}, {shiftt}, {cms}, {masks}, {shifts})"),
            Gfx::SetFogColor(r, g, b, a) => format!("gsDPSetFogColor({r}, {g}, {b}, {a})"),
            Gfx::SetBlendColor(r, g, b, a) => format!("gsDPSetBlendColor({r}, {g}, {b}, {a})"),
            Gfx::SetPrimColor(m, l, r, g, b, a) => format!("gsDPSetPrimColor({m}, {l}, {r}, {g}, {b}, {a})"),
            Gfx::SetEnvColor(r, g, b, a) => format!("gsDPSetEnvColor({r}, {g}, {b}, {a})"),
            Gfx::SetCombine(a0, b0, c0, d0, aa0, ab0, ac0, ad0, a1, b1, c1, d1, aa1, ab1, ac1, ad1) => {
                let [a0, b0, c0, d0, a1, b1, c1, d1] = [a0, b0, c0, d0, a1, b1, c1, d1].map(|x| match x {
                    0 => "COMBINED",
                    1 => "TEXEL0",
                    2 => "TEXEL1",
                    3 => "PRIMITIVE",
                    4 => "SHADE",
                    5 => "ENVIRONMENT",
                    6 => "CENTER",
                    7 => "COMBINED_ALPHA",
                    8 => "TEXEL0_ALPHA",
                    9 => "TEXEL1_ALPHA",
                    10 => "PRIMITIVE_ALPHA",
                    11 => "SHADE_ALPHA",
                    12 => "ENV_ALPHA",
                    13 => "LOD_FRACTION",
                    14 => "PRIM_LOD_FRAC",
                    15 | 31 => "0",
                    x => unimplemented!("Unhandled G_CCMUX {x:#X}"),
                });
                let [aa0, ab0, ac0, ad0, aa1, ab1, ac1, ad1] = [aa0, ab0, ac0, ad0, aa1, ab1, ac1, ad1].map(|x| match x {
                    0 => "COMBINED",
                    1 => "TEXEL0",
                    2 => "TEXEL1",
                    3 => "PRIMITIVE",
                    4 => "SHADE",
                    5 => "ENVIRONMENT",
                    6 => "1",
                    7 => "0",
                    x => unimplemented!("Unhandled G_ACMUX {x:#X}"),
                });

                format!("gsDPSetCombineLERP({a0}, {b0}, {c0}, {d0}, {aa0}, {ab0}, {ac0}, {ad0}, {a1}, {b1}, {c1}, {d1}, {aa1}, {ab1}, {ac1}, {ad1})")
            }
            Gfx::SetTextureImage(f, s, w, i) => format!("gsDPSetTextureImage({f}, {s}, {w}, {})", resolve(i)),
            Gfx::Unhandled(x) => format!("// WARNING!!! unhandled cmd: {x:#018X}"),
        }
    }
}

pub fn parse_script(seg_data: &SegmentData, addr: SegmentedPointer) -> Result<Vec<Gfx>, std::io::Error> {
    let mut ret = vec![];
    let reader = &mut seg_data.get_reader_from_seg_ptr(addr)?;

    let mut read_u64 = || reader.read_u64::<BigEndian>();
    let extract = |val: u64, offset: u64, width: u64| (val >> offset) & ((1 << width) - 1); 
    let mut stop = false;
    while !stop {
        // Not gonna do the padding bits asserts here
        let cmd = read_u64()?;
        let opcode = extract(cmd, 56, 8) as u8;
        ret.push(match opcode {
            0x01 => {
                let n = extract(cmd, 44, 8) as u8;
                Gfx::Vertex(extract(cmd, 0, 32) as u32, n, extract(cmd, 33, 7) as u8 - n)
            }
            0x03 => Gfx::Cull(extract(cmd, 32, 16) as u16 / 2, extract(cmd, 0, 16) as u16 / 2),
            0x05 => Gfx::Triangle(extract(cmd, 48, 8) as u8 / 2, extract(cmd, 40, 8) as u8 / 2, extract(cmd, 32, 8) as u8 / 2),
            0x06 => Gfx::TwoTriangles(
                extract(cmd, 48, 8) as u8 / 2, extract(cmd, 40, 8) as u8 / 2, extract(cmd, 32, 8) as u8 / 2,
                extract(cmd, 16, 8) as u8 / 2, extract(cmd, 8, 8) as u8 / 2, extract(cmd, 0, 8) as u8 / 2
            ),
            0xD7 => Gfx::Texture(
                // s, t
                extract(cmd, 16, 16) as u16, extract(cmd, 0, 16) as u16,
                // level, tile, on
                extract(cmd, 43, 3) as u8, extract(cmd, 40, 3) as u8, extract(cmd, 33, 7) as u8
            ),
            0xD9 => Gfx::GeometryMode(extract(!cmd, 32, 24) as u32, extract(cmd, 0, 32) as u32),
            0xDB => Gfx::MoveWord(extract(cmd, 48, 8) as u8, extract(cmd, 32, 16) as u16, extract(cmd, 0, 32) as u32),
            0xDC => Gfx::MoveMem(extract(cmd, 0, 32) as u32, (extract(cmd, 51, 5) as u16 + 1) * 8, extract(cmd, 32, 8) as u8, extract(cmd, 40, 8) as u16 * 8),
            0xDE => match extract(cmd, 48, 8) as u8 {
                0 => Gfx::DisplayList(extract(cmd, 0, 32) as u32),
                1 => {
                    stop = true;
                    Gfx::BranchList(extract(cmd, 0, 32) as u32)
                }
                x => unimplemented!("Unhandled G_DL command {x:#X}"),
            }
            0xDF => {
                stop = true;
                Gfx::End
            }
            0xE2 | 0xE3 => {
                let len = extract(cmd, 32, 8) + 1;
                let sft = 32 - len - extract(cmd, 40, 8);
                Gfx::SetOtherMode(opcode, sft as u8, len as u8, extract(cmd, 0, 32) as u32)
            }
            0xE6 => Gfx::LoadSync,
            0xE7 => Gfx::PipeSync,
            0xE8 => Gfx::TileSync,
            0xEE => Gfx::SetPrimDepth(extract(cmd, 16, 16) as u16, extract(cmd, 0, 16) as u16),
            0xF0 => Gfx::LoadTLUT(extract(cmd, 24, 3) as u8, extract(cmd, 14, 10) as u16),
            0xF2 => Gfx::SetTileSize(extract(cmd, 24, 3) as u8, extract(cmd, 44, 12) as u16, extract(cmd, 32, 12) as u16, extract(cmd, 12, 12) as u16, extract(cmd, 0, 12) as u16),
            0xF3 => Gfx::LoadBlock(extract(cmd, 24, 3) as u8, extract(cmd, 44, 12) as u16, extract(cmd, 32, 12) as u16, extract(cmd, 12, 12) as u16, extract(cmd, 0, 12) as u16),
            0xF4 => Gfx::LoadTile(extract(cmd, 24, 3) as u8, extract(cmd, 44, 12) as u16, extract(cmd, 32, 12) as u16, extract(cmd, 12, 12) as u16, extract(cmd, 0, 12) as u16),
            0xF5 => Gfx::SetTile(
                // fmt, siz, line, tmem
                extract(cmd, 53, 3) as u8, extract(cmd, 51, 2) as u8, extract(cmd, 41, 9) as u16, extract(cmd, 32, 9) as u16,
                // tile, palette, cmt, maskt
                extract(cmd, 24, 3) as u8, extract(cmd, 20, 4) as u8, extract(cmd, 18, 2) as u8, extract(cmd, 14, 4) as u8,
                // shiftt, cms, masks, shifts
                extract(cmd, 10, 4) as u8, extract(cmd, 8, 2) as u8, extract(cmd, 4, 4) as u8, extract(cmd, 0, 4) as u8,
            ),
            0xF8 => Gfx::SetFogColor(extract(cmd, 24, 8) as u8, extract(cmd, 16, 8) as u8, extract(cmd, 8, 8) as u8, extract(cmd, 0, 8) as u8),
            0xF9 => Gfx::SetBlendColor(extract(cmd, 24, 8) as u8, extract(cmd, 16, 8) as u8, extract(cmd, 8, 8) as u8, extract(cmd, 0, 8) as u8),
            0xFA => Gfx::SetPrimColor(extract(cmd, 40, 8) as u8, extract(cmd, 32, 8) as u8, extract(cmd, 24, 8) as u8, extract(cmd, 16, 8) as u8, extract(cmd, 8, 8) as u8, extract(cmd, 0, 8) as u8),
            0xFB => Gfx::SetEnvColor(extract(cmd, 24, 8) as u8, extract(cmd, 16, 8) as u8, extract(cmd, 8, 8) as u8, extract(cmd, 0, 8) as u8),
            0xFC => {
                // Why
                Gfx::SetCombine(
                    // a0, b0, c0, d0
                    extract(cmd, 52, 4) as u8, extract(cmd, 28, 4) as u8, extract(cmd, 47, 5) as u8, extract(cmd, 15, 3) as u8,
                    // aa0, ab0, ac0, ad0
                    extract(cmd, 44, 3) as u8, extract(cmd, 12, 3) as u8, extract(cmd, 41, 3) as u8, extract(cmd,  9, 3) as u8,
                    // a1, b1, c1, d1
                    extract(cmd, 37, 4) as u8, extract(cmd, 24, 4) as u8, extract(cmd, 32, 5) as u8, extract(cmd,  6, 3) as u8,
                    // aa1, ab1, ac1, ad1
                    extract(cmd, 21, 3) as u8, extract(cmd,  3, 3) as u8, extract(cmd, 18, 3) as u8, extract(cmd,  0, 3) as u8,
                )
            }
            0xFD => Gfx::SetTextureImage(extract(cmd, 53, 3) as u8, extract(cmd, 51, 2) as u8, extract(cmd, 32, 12) as u16 + 1, extract(cmd, 0, 32) as u32),
            _ => Gfx::Unhandled(cmd),
        });
    }

    Ok(ret)
}

pub fn to_c(script: &[Gfx], name: &str, seg_common: &SegmentState, sym: &SymbolState) -> Result<String, std::io::Error>  {
    let mut buf = BufWriter::new(Vec::new());

    writeln!(buf, "Gfx {name}[] = {{")?;
    for instr in script {
        writeln!(buf, "    {},", instr.to_c(seg_common, sym))?;
    }
    writeln!(buf, "}};")?;

    Ok(String::from_utf8(buf.into_inner()?).unwrap())
}