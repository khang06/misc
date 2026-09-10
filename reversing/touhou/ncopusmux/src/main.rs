use std::{
    fs::File,
    io::{self, BufReader, Read, Seek},
};

use bytemuck::{Pod, Zeroable};
use byteorder::{BigEndian, ReadBytesExt};
use opus_pure::{OggOpusWriter, OpusHead};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct OpusHeader {
    pub magic: u32,                   // 0x0
    pad4: [u8; 5],                    // 0x4
    pub channels: u8,                 // 0x9
    pub decoded_frame_len_bytes: i16, // 0xA
    pub sample_rate: u32,             // 0xC
    pub header2_pos: u32,             // 0x10
    pad2: [u8; 8],                    // 0x14
    pub start_trim: i16,              // 0x1C
    pad1e: [u8; 6],                   // 0x1E
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct OpusHeader2 {
    pub magic: u32,            // 0x0
    pub total_size_bytes: u32, // 0x4
}

fn read_struct<T: Pod>(r: &mut impl Read) -> io::Result<T> {
    let mut value = T::zeroed();
    r.read_exact(bytemuck::bytes_of_mut(&mut value))?;
    Ok(value)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <input.opus> <output.opus>", args[0]);
        std::process::exit(1);
    }

    let mut input = BufReader::new(File::open(&args[1])?);

    let header1: OpusHeader = read_struct(&mut input)?;
    if header1.magic != 0x80000001 {
        return Err(format!("Invalid OpusHeader magic: 0x{:x}", header1.magic).into());
    }
    if header1.channels != 1 && header1.channels != 2 {
        return Err(format!("Invalid OpusHeader channels: {}", header1.channels).into());
    }
    if header1.sample_rate != 48000 {
        return Err(format!("Invalid OpusHeader sample rate: {}", header1.sample_rate).into());
    }

    // I have no idea why this header exists
    input.seek(io::SeekFrom::Start(header1.header2_pos as u64))?;
    let header2: OpusHeader2 = read_struct(&mut input)?;
    if header2.magic != 0x80000004 {
        return Err(format!("Invalid OpusHeader2 magic: 0x{:x}", header2.magic).into());
    }

    let mux_head = OpusHead {
        pre_skip: header1.start_trim as u16,
        ..OpusHead::new(header1.channels, header1.sample_rate)?
    };

    let output = File::create(&args[2])?;
    let mut writer = OggOpusWriter::new(output, mux_head)?;
    let mut scratch = vec![];
    let mut frames = 0;
    while let Ok(len) = input.read_u32::<BigEndian>() {
        // For some reason there's 4 bytes of padding here? It only reads 4 bytes for the length...
        input.seek_relative(4)?;

        scratch.resize((len as usize).max(scratch.len()), 0);
        input.read_exact(&mut scratch[..len as usize])?;
        writer.write_packet_with_duration(&scratch[..len as usize], 960)?;
        frames += 1;
    }
    writer.finish()?;

    println!("Muxed {frames} frames");

    Ok(())
}
