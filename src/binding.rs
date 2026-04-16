use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use crate::{
    capture::{VideoCapture, VideoCaptureError},
    decoder::{DecoderError, HardwareAcceleration, VideoDecoder},
    types::Packet,
};
use pyo3::exceptions::{PyConnectionError, PyIOError, PyValueError};
use pyo3::prelude::*;

type PacketBuffer = VecDeque<Packet>;

#[pyclass]
struct RsVideoCapture {
    buffer: Arc<Mutex<PacketBuffer>>,
    decoder: Mutex<VideoDecoder>,
    width: u32,
    height: u32,
    is_closed: Arc<AtomicBool>,
}

impl RsVideoCapture {
    fn connect(
        path: &str,
        timeout: u32,
        hardware_acceleration: HardwareAcceleration,
    ) -> Result<(VideoCapture, VideoDecoder), PyErr> {
        let (capture, codec) = VideoCapture::new(path, timeout).map_err(|err| match err {
            VideoCaptureError::FailedToOpenFile => {
                PyConnectionError::new_err(format!("Failed to open video source: {}", path))
            }
            VideoCaptureError::FailedToFindVideo => {
                PyConnectionError::new_err("No video stream found in source")
            }
            VideoCaptureError::ErrReadingFile => {
                PyIOError::new_err("Error reading from video source")
            }
        })?;
        let decoder =
            VideoDecoder::new(codec, capture.codecpar(), hardware_acceleration).map_err(|err| {
                match err {
                    DecoderError::UnsupportedPlatform => PyValueError::new_err(
                        "Selected hardware acceleration is not supported on this platform",
                    ),
                    DecoderError::FailedToOpenDecoder => {
                        PyValueError::new_err("Failed to open decoder")
                    }
                    DecoderError::NoHwConfig => {
                        PyValueError::new_err("No hardware decoder config found for codec")
                    }
                }
            })?;
        Ok((capture, decoder))
    }
}

#[pymethods]
impl RsVideoCapture {
    #[new]
    #[pyo3(signature = (path, /, *, timeout=10000, hardware_acceleration=None))]
    pub fn new(
        path: String,
        timeout: u32,
        hardware_acceleration: Option<HardwareType>,
    ) -> PyResult<Self> {
        let (mut capture, decoder) = Self::connect(
            &path,
            timeout,
            hardware_acceleration
                .map(|hw| hw.into())
                .unwrap_or(HardwareAcceleration::None),
        )?;
        let buffer = Arc::new(Mutex::new(PacketBuffer::new()));
        let is_closed = Arc::new(AtomicBool::new(false));
        let instance = RsVideoCapture {
            buffer: buffer.clone(),
            width: decoder.width() as u32,
            height: decoder.height() as u32,
            decoder: Mutex::new(decoder),
            is_closed: is_closed.clone(),
        };

        thread::spawn(move || {
            while !is_closed.load(Ordering::Relaxed) {
                let packet = match capture.receive() {
                    Ok(Some(packet)) => packet,
                    _ => break,
                };
                let mut buffer = buffer.lock().unwrap();
                if packet.flags == 1 {
                    buffer.clear();
                }
                buffer.push_back(packet);
            }
            is_closed.store(true, Ordering::Relaxed);
        });

        Ok(instance)
    }

    pub fn grab(&mut self) -> PyResult<Option<Vec<u8>>> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(PyConnectionError::new_err("Connection is closed"));
        }
        let mut decoder = self.decoder.lock().unwrap();
        let packets: Vec<Packet> = {
            let mut buffer = self.buffer.lock().unwrap();
            buffer.drain(..).collect()
        };
        let frames = packets
            .into_iter()
            .flat_map(|packet| decoder.decode(&packet));
        Ok(frames.last())
    }

    pub fn close(&mut self) {
        self.is_closed.store(true, Ordering::Relaxed);
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

#[pyclass(eq, eq_int)]
#[derive(PartialEq, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum HardwareType {
    VAAPI,
    VideoToolbox,
    D3D11VA,
    D3D12VA,
    CUDA,
    Vulkan,
}

impl From<HardwareType> for HardwareAcceleration {
    fn from(value: HardwareType) -> Self {
        match value {
            HardwareType::VAAPI => HardwareAcceleration::VAAPI,
            HardwareType::VideoToolbox => HardwareAcceleration::VideoToolbox,
            HardwareType::D3D11VA => HardwareAcceleration::D3D11VA,
            HardwareType::D3D12VA => HardwareAcceleration::D3D12VA,
            HardwareType::CUDA => HardwareAcceleration::CUDA,
            HardwareType::Vulkan => HardwareAcceleration::Vulkan,
        }
    }
}

#[pymodule]
fn video_capture(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RsVideoCapture>()?;
    m.add_class::<HardwareType>()?;
    Ok(())
}
