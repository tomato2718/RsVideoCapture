mod hardware;
mod software;
use crate::types::{Packet, VideoCaptureError};
use rsmpeg::avcodec::{AVCodecParametersRef, AVCodecRef};
use rsmpeg::ffi;

const ERR_UNSUPPORTED_PLATFORM: VideoCaptureError =
    "Hardware decoding not supported on this platform";

pub enum VideoDecoder {
    Software(software::SoftwareDecoder),
    Hardware(hardware::HardwareDecoder),
}

impl VideoDecoder {
    pub fn new_software(
        codec: AVCodecRef,
        codecpar: AVCodecParametersRef,
    ) -> Result<Self, VideoCaptureError> {
        software::SoftwareDecoder::new(codec, codecpar).map(Self::Software)
    }

    pub fn new_hardware(
        codec: AVCodecRef,
        codecpar: AVCodecParametersRef,
    ) -> Result<Self, VideoCaptureError> {
        let device_type = Self::default_hw_device_type()?;
        hardware::HardwareDecoder::new(codec, codecpar, device_type).map(Self::Hardware)
    }

    fn default_hw_device_type() -> Result<ffi::AVHWDeviceType, VideoCaptureError> {
        #[cfg(target_os = "macos")]
        {
            Ok(ffi::AV_HWDEVICE_TYPE_VIDEOTOOLBOX)
        }
        #[cfg(target_os = "linux")]
        {
            Ok(ffi::AV_HWDEVICE_TYPE_VAAPI)
        }
        #[cfg(target_os = "windows")]
        {
            Ok(ffi::AV_HWDEVICE_TYPE_D3D11VA)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(ERR_UNSUPPORTED_PLATFORM)
        }
    }

    pub fn width(&self) -> usize {
        match self {
            Self::Software(d) => d.width(),
            Self::Hardware(d) => d.width(),
        }
    }

    pub fn height(&self) -> usize {
        match self {
            Self::Software(d) => d.height(),
            Self::Hardware(d) => d.height(),
        }
    }

    pub fn decode(&mut self, packet: &Packet) -> Vec<Vec<u8>> {
        match self {
            Self::Software(d) => d.decode(packet),
            Self::Hardware(d) => d.decode(packet),
        }
    }
}
