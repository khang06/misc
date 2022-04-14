// BASS_Mixer_StreamCreate flags
pub const BASS_MIXER_END: u32 = 0x10000;
pub const BASS_MIXER_NONSTOP: u32 = 0x20000;
pub const BASS_MIXER_RESUME: u32 = 0x1000;
pub const BASS_MIXER_POSEX: u32 = 0x2000;

// BASS_Mixer_StreamAddChannel/Ex flags
pub const BASS_MIXER_CHAN_ABSOLUTE: u32 = 0x1000;
pub const BASS_MIXER_CHAN_BUFFER: u32 = 0x2000;
pub const BASS_MIXER_CHAN_LIMIT: u32 = 0x4000;
pub const BASS_MIXER_CHAN_MATRIX: u32 = 0x10000;
pub const BASS_MIXER_CHAN_PAUSE: u32 = 0x20000;
pub const BASS_MIXER_CHAN_DOWNMIX: u32 = 0x400000;
pub const BASS_MIXER_CHAN_NORAMPIN: u32 = 0x800000;
