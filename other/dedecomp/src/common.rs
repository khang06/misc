use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    io::{Cursor, Seek, SeekFrom},
    ops::{BitAnd, Sub},
    ptr::addr_of_mut,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentedPointer {
    Ram(u32),
    RomRaw(u32, u32),
    RomMIO0(u32, u32),
}

impl Default for SegmentedPointer {
    fn default() -> Self {
        SegmentedPointer::Ram(0)
    }
}

#[derive(Clone, Copy)]
pub enum SegmentBase {
    Raw(u32),
    MIO0(u32),
}

#[derive(Default, Clone)]
pub struct SegmentState {
    bases: [Option<SegmentBase>; 24],
}

impl SegmentState {
    pub fn set_seg_base(&mut self, idx: usize, base: SegmentBase) {
        assert!(idx < 24, "invalid segment base");
        self.bases[idx] = Some(base);
    }

    pub fn addr_to_seg_ptr(&self, addr: u32) -> SegmentedPointer {
        let base_idx = addr >> 24;
        if base_idx == 0x80 {
            SegmentedPointer::Ram(addr)
        } else if addr == 0 {
            SegmentedPointer::Ram(0)
        } else {
            assert!(base_idx < 24, "invalid segment base {base_idx:#X}");
            let base = self.bases[base_idx as usize]
                .unwrap_or_else(|| panic!("uninitialized segment base {base_idx:#X}"));
            let offset = addr & 0xFFFFFF;
            match base {
                SegmentBase::Raw(addr) => SegmentedPointer::RomRaw(addr, offset),
                SegmentBase::MIO0(addr) => SegmentedPointer::RomMIO0(addr, offset),
            }
        }
    }
}

#[derive(Default)]
pub struct SymbolState {
    ptr_map: HashMap<SegmentedPointer, String>,
    skip_decomp_addrs: HashSet<SegmentedPointer>,
}

impl SymbolState {
    pub fn add_sym(&mut self, addr: SegmentedPointer, sym: &str, skip_decomp: bool) {
        self.ptr_map.insert(addr, sym.to_string());
        if skip_decomp {
            self.skip_decomp_addrs.insert(addr);
        }
    }

    pub fn resolve_seg_ptr(&self, addr: SegmentedPointer) -> String {
        self.ptr_map
            .get(&addr)
            .cloned()
            .unwrap_or_else(|| match addr {
                SegmentedPointer::Ram(addr) => {
                    if addr == 0 {
                        "NULL".to_string()
                    } else {
                        format!("_ram_{addr:#X}")
                    }
                }
                SegmentedPointer::RomRaw(base, offset) => format!("_raw_{base:#X}_{offset:#X}"),
                SegmentedPointer::RomMIO0(base, offset) => format!("_mio0_{base:#X}_{offset:#X}"),
            })
    }

    pub fn resolve_raw_ptr(&self, seg_state: &SegmentState, addr: u32) -> String {
        format!(
            "/* {addr:#X} */ {}",
            self.resolve_seg_ptr(seg_state.addr_to_seg_ptr(addr))
        )
    }

    pub fn get_skip_addrs(&self) -> HashSet<SegmentedPointer> {
        self.skip_decomp_addrs.clone()
    }
}

pub struct SegmentData {
    rom: &'static [u8],
    mio0_data: HashMap<u32, Vec<u8>>,
}

impl SegmentData {
    pub fn new(rom: &'static [u8]) -> SegmentData {
        Self {
            rom,
            mio0_data: Default::default(),
        }
    }

    pub fn load_mio0(&mut self, start: u32, end: u32) {
        if self.mio0_data.contains_key(&start) {
            return;
        }

        assert!(start <= end);
        assert!(start < self.rom.len() as u32);
        assert!(end < self.rom.len() as u32);

        let data = unsafe {
            // Serene Fusion doesn't actually use MIO0! It uses something called RNC instead...
            extern "C" {
                fn rnc_unpack(src: *const u8, src_len: u32, dst: *mut u8, dst_len: *mut u32)
                    -> u32;
            }

            let mut dst = vec![0; 0x1E00000];
            let mut dst_len = 0;
            assert_eq!(
                rnc_unpack(
                    self.rom.as_ptr().add(start as usize),
                    end - start,
                    dst.as_mut_ptr(),
                    addr_of_mut!(dst_len)
                ),
                0
            );
            dst.truncate(dst_len as usize);
            dst.shrink_to_fit();

            //std::fs::write(format!("dump/mio0_{start:#X}.bin"), &dst).unwrap();

            dst
        };
        self.mio0_data.insert(start, data);
    }

    pub fn get_rom_reader(&self) -> Cursor<&[u8]> {
        Cursor::new(self.rom)
    }

    pub fn get_mio0_reader(&self, addr: u32) -> Cursor<&[u8]> {
        Cursor::new(
            self.mio0_data
                .get(&addr)
                .unwrap_or_else(|| panic!("invalid mio0 pointer {addr:#X}")),
        )
    }

    pub fn get_reader_from_seg_ptr(
        &self,
        addr: SegmentedPointer,
    ) -> Result<Cursor<&[u8]>, std::io::Error> {
        match addr {
            SegmentedPointer::Ram(_) => panic!("Can't parse script from RAM"),
            SegmentedPointer::RomRaw(base, offset) => {
                let mut ret = self.get_rom_reader();
                ret.seek(SeekFrom::Start((base + offset) as u64))?;
                Ok(ret)
            }
            SegmentedPointer::RomMIO0(base, offset) => {
                let mut ret = self.get_mio0_reader(base);
                ret.seek(SeekFrom::Start(offset as u64))?;
                Ok(ret)
            }
        }
    }
}

// HackerSM64's silhouette feature shifts some layer indices
// sm64coopdx doesn't have this, so the layers have to be shifted back
pub fn fix_layer<T: BitAnd<Output = T> + Sub<Output = T> + TryInto<u8> + From<u8>>(layer: T) -> T
where
    <T as TryInto<u8>>::Error: Debug,
{
    let layer: u8 = layer.try_into().expect("invalid layer");
    T::from(
        (layer & 0xF0)
            | match layer & 0xF {
                0..=4 => layer & 0xF,
                10..=12 => (layer & 0xF) - 5,

                5 => 1, // LAYER_ALPHA_DECAL
                9 => 1, // LAYER_OCCLUDE_SILHOUETTE_ALPHA
                x => {
                    println!("WARNING!!! unhandled layer {x}");
                    1
                }
            },
    )
}
