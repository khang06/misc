#![feature(thread_sleep_until)]

use std::time::{Duration, Instant};

use windows::{
    core::w, Win32::{
        Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE},
        Storage::FileSystem::{
            CreateFileW, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING
        }, System::IO::DeviceIoControl,
    }
};

enum WoMicIoctl {
    ChallengeRequest = 0x222004,
    ChallengeResponse = 0x222008,
    GetVersion = 0x22200C,
    Start = 0x222010,
    SetBufferSize = 0x222014,
}

struct WoMicClient {
    handle: HANDLE,
}

impl WoMicClient {
    pub fn new() -> Result<Self, WoMicError> {
        let handle = Self::driver_open()?;
        Ok(Self { handle })
    }

    fn driver_open() -> Result<HANDLE, WoMicError> {
        unsafe {
            let Ok(handle) = CreateFileW(
                w!("\\\\.\\WoMic"),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) else {
                return Err(WoMicError::OpenFailed);
            };
            
            let mut challenge = 0u8;
            if DeviceIoControl(handle, WoMicIoctl::ChallengeRequest as u32, None, 0, Some(&mut challenge as *mut u8 as *mut _), 1, None, None).is_err() {
                return Err(WoMicError::OpenFailed);
            }
            challenge ^= 0x3B;
            if DeviceIoControl(handle, WoMicIoctl::ChallengeResponse as u32, Some(&challenge as *const u8 as *const _), 1, None, 0, None, None).is_err() {
                return Err(WoMicError::OpenFailed);
            }

            let mut version = 0u8;
            if DeviceIoControl(handle, WoMicIoctl::GetVersion as u32, None, 0, Some(&mut version as *mut u8 as *mut _), 1, None, None).is_err() {
                return Err(WoMicError::OpenFailed);
            }
            if version != 3 {
                return Err(WoMicError::VersionMismatch);
            }
            
            let size = (1920u16 * 3).to_be_bytes();
            if DeviceIoControl(handle, WoMicIoctl::SetBufferSize as u32, Some(size.as_ptr() as *const _), 1, None, 0, None, None).is_err() {
                return Err(WoMicError::OpenFailed);
            }
            
            let reset = 1u8;
            if DeviceIoControl(handle, WoMicIoctl::Start as u32, Some(&reset as *const u8 as *const _), 1, None, 0, None, None).is_err() {
                return Err(WoMicError::OpenFailed);
            }

            Ok(handle)
        }
    }
    
    fn write_data(&self, data: &[i16]) -> Result<(), WoMicError> {
        unsafe {
            if WriteFile(self.handle, Some(bytemuck::cast_slice(data)), None, None).is_err() {
                return Err(WoMicError::WriteFailed);
            }
        }
        
        Ok(())
    }
}

#[derive(Debug)]
enum WoMicError {
    OpenFailed,
    VersionMismatch,
    WriteFailed,
}

fn main() -> Result<(), WoMicError> {
    let client = WoMicClient::new()?;
    
    let mut buffer = [0i16; 960];
    let mut target = Instant::now();
    let mut sample_clock = 0f32;
    loop {
        for i in 0..buffer.len() {
            buffer[i] = ((sample_clock * 440.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 32767.0) as i16;
            sample_clock = (sample_clock + 1.0) % 48000.0;
        }
        client.write_data(&buffer)?;

        while target < Instant::now() {
            target += Duration::from_millis(20);
        }
        std::thread::sleep_until(target);
    }
}
