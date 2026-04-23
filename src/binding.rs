use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;

use crate::{
    capture::{VideoCapture, VideoCaptureError},
    decoder::{DecoderError, HardwareAcceleration, VideoDecoder},
    packet::{clone_packet, Packet},
    writer::VideoWriter,
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
    daemon_threads: Vec<thread::JoinHandle<()>>,
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

    fn setup_capture_thread(
        &mut self,
        mut capture: VideoCapture,
        writer: Option<mpsc::Sender<Packet>>,
    ) {
        let is_closed = self.is_closed.clone();
        let buffer = self.buffer.clone();
        let mut tasks: Vec<Box<dyn Fn(Packet) + Send>> = vec![];
        tasks.push(Box::new(move |packet| {
            let mut buffer = buffer.lock().unwrap();
            if packet.flags == 1 {
                buffer.clear();
            }
            buffer.push_back(packet);
        }));
        if let Some(writer) = writer {
            let is_closed = self.is_closed.clone();
            tasks.push(Box::new(move |packet| {
                if writer.send(packet).is_err() {
                    is_closed.store(true, Ordering::Relaxed);
                }
            }))
        }
        let handler = thread::spawn(move || {
            while let Ok(Some(packet)) = capture.receive() {
                if packet.flags == 1 {
                    for task in tasks.iter() {
                        task(clone_packet(&packet));
                    }
                    break;
                }
            }
            while !is_closed.load(Ordering::Relaxed) {
                let packet = match capture.receive() {
                    Ok(Some(packet)) => packet,
                    _ => break,
                };
                for task in tasks.iter() {
                    task(clone_packet(&packet));
                }
            }
            is_closed.store(true, Ordering::Relaxed);
        });
        self.daemon_threads.push(handler);
    }

    fn setup_writer_thread(
        &mut self,
        path: String,
        capture: &VideoCapture,
    ) -> Result<mpsc::Sender<Packet>, PyErr> {
        let mut writer = VideoWriter::new(&path, capture.codecpar().clone(), capture.time_base())
            .map_err(|_| PyIOError::new_err("Failed to open writer"))?;
        let (tx, rx) = mpsc::channel();
        let handler = thread::spawn(move || {
            for packet in rx.iter() {
                if writer.push(packet).is_err() {
                    break;
                };
            }
            let _ = writer.end();
        });
        self.daemon_threads.push(handler);

        Ok(tx)
    }
}

#[pymethods]
impl RsVideoCapture {
    #[new]
    #[pyo3(signature = (path, /, *, timeout=10000, hardware_acceleration=None, save_path=None))]
    pub fn new(
        path: String,
        timeout: u32,
        hardware_acceleration: Option<HardwareType>,
        save_path: Option<String>,
    ) -> PyResult<Self> {
        let (capture, decoder) = Self::connect(
            &path,
            timeout,
            hardware_acceleration
                .map(|hw| hw.into())
                .unwrap_or(HardwareAcceleration::None),
        )?;
        let mut instance = RsVideoCapture {
            buffer: Arc::new(Mutex::new(PacketBuffer::new())),
            width: decoder.width() as u32,
            height: decoder.height() as u32,
            decoder: Mutex::new(decoder),
            is_closed: Arc::new(AtomicBool::new(false)),
            daemon_threads: Vec::new(),
        };

        let writer_tx = save_path
            .map(|path| instance.setup_writer_thread(path, &capture))
            .transpose()?;
        instance.setup_capture_thread(capture, writer_tx);

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
        for t in self.daemon_threads.drain(..) {
            t.join().expect("Couldn't join on the associated thread");
        }
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
