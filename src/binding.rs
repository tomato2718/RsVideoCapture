use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use crate::{connect, decoder::VideoDecoder, types::Packet};
use pyo3::exceptions::PyException;
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

#[pymethods]
impl RsVideoCapture {
    #[new]
    #[pyo3(signature = (path, /, *, timeout=10000, use_hardware=false))]
    pub fn new(path: String, timeout: u32, use_hardware: bool) -> PyResult<Self> {
        let (mut capture, decoder) = match connect(&path, timeout, use_hardware) {
            Ok(res) => res,
            Err(e) => return Err(PyException::new_err(e)),
        };
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

    pub fn grab(&mut self) -> PyResult<Vec<u8>> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(PyException::new_err("Connection is closed"));
        }
        let mut decoder = self.decoder.lock().unwrap();
        let packets: Vec<Packet> = {
            let mut buffer = self.buffer.lock().unwrap();
            buffer.drain(..).collect()
        };
        let frames = packets
            .into_iter()
            .flat_map(|packet| decoder.decode(&packet));
        match frames.last() {
            Some(frame) => Ok(frame),
            None => Err(PyException::new_err("No frame received")),
        }
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

#[pymodule]
fn video_capture(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RsVideoCapture>()?;
    Ok(())
}
