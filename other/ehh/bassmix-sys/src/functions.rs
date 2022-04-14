// based on bass-sys
/*
MIT License

Copyright (c) 2020 KernelError

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use std::env;

use libloading::Library;
use once_cell::sync::Lazy;

static BASSMIX_LIBRARY: Lazy<Library> = Lazy::new(|| {
    if let Ok(mut library_path) = env::current_exe() {
        library_path.pop();

        #[cfg(target_os = "windows")]
        library_path.push("bassmix.dll");

        #[cfg(target_os = "linux")]
        library_path.push("libbassmix.so");

        #[cfg(target_os = "macos")]
        library_path.push("libbassmix.dylib");

        if let Ok(library) = unsafe { Library::new(library_path) } {
            library
        } else {
            panic!("Failed to load BASSmix.");
        }
    } else {
        panic!("Failed to load BASSmix.");
    }
});

macro_rules! generate_bindings {
    ($(binding $static_name:ident fn $binding_name:ident($($parameter_name:ident:$parameter_type:ty),*) $(-> $return_type:ty)?;)*) => {
        $(
            static $static_name: once_cell::sync::Lazy<libloading::Symbol<'static, extern fn($($parameter_name: $parameter_type),*) $(-> $return_type)?>> = once_cell::sync::Lazy::new(|| {
                if let Ok(function) = unsafe { BASSMIX_LIBRARY.get(stringify!($binding_name).as_bytes()) } {
                    return function;
                } else {
                    panic!("Failed to load the function.");
                }
            });

            #[allow(non_snake_case)]
            pub fn $binding_name($($parameter_name: $parameter_type),*) $(-> $return_type)? {
                $static_name($($parameter_name),*)
            }
        )*
    };
}

generate_bindings! {
    binding BASS_MIXER_GETVERSION fn BASS_Mixer_GetVersion() -> u32;
    binding BASS_MIXER_STREAMCREATE fn BASS_Mixer_StreamCreate(freq: u32, chans: u32, flags: u32) -> bass_sys::HSTREAM;
    binding BASS_MIXER_STREAMADDCHANNEL fn BASS_Mixer_StreamAddChannel(handle: bass_sys::HSTREAM, channel: u32, flags: u32) -> i32;
    binding BASS_MIXER_CHANNELGETPOSITION fn BASS_Mixer_ChannelGetPosition(handle: bass_sys::HSTREAM, mode: u32) -> u64;
    binding BASS_MIXER_CHANNELSETPOSITION fn BASS_Mixer_ChannelSetPosition(handle: bass_sys::HSTREAM, pos: u64, flag: u32) -> bool;
    binding BASS_MIXER_CHANNELISACTIVE fn BASS_Mixer_ChannelIsActive(handle: bass_sys::HSTREAM) -> u32;
    binding BASS_MIXER_CHANNELFLAGS fn BASS_Mixer_ChannelFlags(handle: bass_sys::HSTREAM, flags: u32, mask: u32) -> u32;
}
