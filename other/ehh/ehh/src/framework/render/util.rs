use gl::types::*;

pub fn vertical_flip_texture(data: &mut [u8], width: usize, height: usize, bpp: usize) {
    assert!(data.len() == width * height * bpp);
    let row_size = width * bpp;
    let mut chunks = data.chunks_exact_mut(row_size);
    while let (Some(front), Some(back)) = (chunks.next(), chunks.next_back()) {
        front.swap_with_slice(back);
    }
}

pub fn format_to_bpp(format: GLenum) -> u32 {
    match format {
        gl::RGBA | gl::RGBA8 => 4,
        gl::RGB => 3,
        gl::RG => 2,
        gl::RED => 1,
        x => panic!("Unknown format {x}"),
    }
}

pub fn format_to_internal(format: GLenum) -> GLenum {
    match format {
        gl::RGBA => gl::RGBA8,
        gl::RGB => gl::RGB8,
        x => panic!("Unknown format {:x}", x),
    }
}

#[cfg(test)]
mod tests {
    use rgb::ComponentBytes;

    use crate::framework::render::util::vertical_flip_texture;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_vflip() {
        let mut input = lodepng::decode32_file("test/test.png").unwrap();
        let output = lodepng::decode32_file("test/test_flip.png").unwrap();
        assert_eq!(input.width, output.width);
        assert_eq!(input.height, output.height);

        vertical_flip_texture(input.buffer.as_bytes_mut(), input.width, input.height, 4);

        for (x, y) in input.buffer.iter().zip(output.buffer.iter()) {
            assert_eq!(x, y);
        }
    }
}
