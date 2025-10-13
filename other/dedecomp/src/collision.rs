use std::io::{BufWriter, Write};

use byteorder::{BigEndian, ReadBytesExt};

use crate::common::{SegmentData, SegmentedPointer};

pub struct CollisionTriGroup {
    surface: i16,
    triangles: Vec<(i16, i16, i16, i16)>,
}

/*
pub enum SpecialObj {
    NoYRotOrParams(i16, i16, i16, i16),
    YRotNoParams(i16, i16, i16, i16, i16),
    ParamsAndYRot(i16, i16, i16, i16, i16, i16),
    DefParamAndYRot(i16, i16, i16, i16, i16),
}
*/

pub struct Collision {
    vertices: Vec<(i16, i16, i16)>,
    groups: Vec<CollisionTriGroup>,
    //special_objs: Vec<SpecialObj>,
    waterboxes: Vec<(i16, i16, i16, i16, i16, i16)>,
}

impl Collision {
    pub fn to_c(&self, name: &str) -> Result<String, std::io::Error> {
        let mut buf = BufWriter::new(Vec::new());

        writeln!(buf, "Collision {name}[] = {{\n    COL_INIT(),\n    COL_VERTEX_INIT({})", self.vertices.len())?;
        for vertex in &self.vertices {
            writeln!(buf, "    COL_VERTEX({}, {}, {}),", vertex.0, vertex.1, vertex.2)?;
        }
        for group in &self.groups {
            writeln!(buf, "    COL_TRI_INIT({}, {}),", group.surface, group.triangles.len())?;
    
            let has_force = matches!(group.surface, 0x4 | 0xE | 0x2C | 0x27 | 0x24 | 0x25 | 0x2D);
            if has_force {
                for tri in &group.triangles {
                    writeln!(buf, "    COL_TRI_SPECIAL({}, {}, {}, {}),", tri.0, tri.1, tri.2, tri.3)?;
                }
            } else {
                for tri in &group.triangles {
                    if tri.3 != 0 {
                        writeln!(buf, "    // WARNING!!! non-zero force: {:#X}", tri.3)?;
                    }
                    writeln!(buf, "    COL_TRI({}, {}, {}),", tri.0, tri.1, tri.2)?;
                }
            }
        }
        writeln!(buf, "    COL_TRI_STOP(),")?;
        if !self.waterboxes.is_empty() {
            writeln!(buf, "    COL_WATER_BOX_INIT({}),", self.waterboxes.len())?;
            for waterbox in &self.waterboxes {
                writeln!(buf, "    COL_WATER_BOX({}, {}, {}, {}, {}, {}),", waterbox.0, waterbox.1, waterbox.2, waterbox.3, waterbox.4, waterbox.5)?;
            }
        }
        writeln!(buf, "    COL_END(),\n}};")?;

        Ok(String::from_utf8(buf.into_inner()?).unwrap())
    }

    pub fn tri_count(&self) -> usize {
        self.groups.iter().fold(0, |acc, x| acc + x.triangles.len())
    }
}

pub fn parse_script(seg_data: &SegmentData, addr: SegmentedPointer) -> Result<Collision, std::io::Error> {
    const TERRAIN_LOAD_VERTICES: i16 = 0x40;
    const TERRAIN_LOAD_CONTINUE: i16 = 0x41;
    const TERRAIN_LOAD_END: i16 = 0x42;
    //const TERRAIN_LOAD_OBJECTS: i16 = 0x43;
    const TERRAIN_LOAD_ENVIRONMENT: i16 = 0x44;
    
    let mut vertices = vec![];
    let mut groups = vec![];
    //let mut special_objs = vec![];
    let mut waterboxes = vec![];

    let reader = &mut seg_data.get_reader_from_seg_ptr(addr)?;
    let mut read_i16 = || reader.read_i16::<BigEndian>();
    loop {
        let cmd = read_i16()?;
        match cmd {
            TERRAIN_LOAD_VERTICES => {
                let count = read_i16()?.try_into().expect("invalid count");
                vertices.reserve(count);
                for _ in 0..count {
                    vertices.push((read_i16()?, read_i16()?, read_i16()?));
                }

                loop {
                    let surface = read_i16()?;
                    if surface == TERRAIN_LOAD_CONTINUE {
                        break;
                    } else {
                        let count = read_i16()?.try_into().expect("invalid count");
                        let mut triangles = Vec::with_capacity(count);
                        for _ in 0..count {
                            triangles.push((read_i16()?, read_i16()?, read_i16()?, read_i16()?));
                        }

                        groups.push(CollisionTriGroup {
                            surface,
                            triangles,
                        })
                    }
                }
            }
            TERRAIN_LOAD_ENVIRONMENT => {
                let count = read_i16()?.try_into().expect("invalid count");
                waterboxes.reserve(count);
                for _ in 0..count {
                    waterboxes.push((read_i16()?, read_i16()?, read_i16()?, read_i16()?, read_i16()?, read_i16()?));
                }
            }
            TERRAIN_LOAD_END => break,
            x => unimplemented!("Unhandled command {x:#X}"),
        }
    }

    Ok(Collision {
        vertices,
        groups,
        //special_objs,
        waterboxes,
    })
}
