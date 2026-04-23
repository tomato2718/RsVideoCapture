mod hardware;
mod software;
use crate::packet::Packet;
use rsmpeg::avcodec::{AVCodecParametersRef, AVCodecRef};
use rsmpeg::ffi;

pub enum VideoDecoder {
    Software(software::SoftwareDecoder),
    Hardware(hardware::HardwareDecoder),
}

impl VideoDecoder {
    pub fn new(
        codec: AVCodecRef,
        codecpar: AVCodecParametersRef,
        hardware_acceleration: HardwareAcceleration,
    ) -> Result<Self, DecoderError> {
        match hardware_acceleration {
            HardwareAcceleration::None => {
                software::SoftwareDecoder::new(codec, codecpar).map(Self::Software)
            }
            #[cfg(target_os = "macos")]
            HardwareAcceleration::VideoToolbox => {
                hardware::HardwareDecoder::new(codec, codecpar, ffi::AV_HWDEVICE_TYPE_VIDEOTOOLBOX)
                    .map(Self::Hardware)
            }
            #[cfg(target_os = "linux")]
            HardwareAcceleration::VAAPI => {
                hardware::HardwareDecoder::new(codec, codecpar, ffi::AV_HWDEVICE_TYPE_VAAPI)
                    .map(Self::Hardware)
            }
            #[cfg(target_os = "windows")]
            HardwareAcceleration::D3D11VA => {
                hardware::HardwareDecoder::new(codec, codecpar, ffi::AV_HWDEVICE_TYPE_D3D11VA)
                    .map(Self::Hardware)
            }
            #[cfg(target_os = "windows")]
            HardwareAcceleration::D3D12VA => {
                hardware::HardwareDecoder::new(codec, codecpar, ffi::AV_HWDEVICE_TYPE_D3D12VA)
                    .map(Self::Hardware)
            }
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            HardwareAcceleration::CUDA => {
                hardware::HardwareDecoder::new(codec, codecpar, ffi::AV_HWDEVICE_TYPE_CUDA)
                    .map(Self::Hardware)
            }
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            HardwareAcceleration::Vulkan => {
                hardware::HardwareDecoder::new(codec, codecpar, ffi::AV_HWDEVICE_TYPE_VULKAN)
                    .map(Self::Hardware)
            }
            _ => Err(DecoderError::UnsupportedPlatform),
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

#[allow(clippy::upper_case_acronyms)]
pub enum HardwareAcceleration {
    None,
    VAAPI,
    VideoToolbox,
    D3D11VA,
    D3D12VA,
    CUDA,
    Vulkan,
}

pub enum DecoderError {
    UnsupportedPlatform,
    FailedToOpenDecoder,
    NoHwConfig,
}
