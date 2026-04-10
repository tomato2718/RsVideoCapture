use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use crate::core::{connect, Packet, VideoDecoder};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

type PacketBuffer = VecDeque<Packet>;

#[pyclass]
struct RsVideoCapture {
    buffer: Arc<Mutex<PacketBuffer>>,
    decoder: Mutex<VideoDecoder>,
    closed: Arc<AtomicBool>,
}

#[pymethods]
impl RsVideoCapture {
    #[new]
    pub fn new(path: String) -> PyResult<Self> {
        let (mut capture, decoder) = match connect(&path) {
            Ok(res) => res,
            Err(e) => return Err(PyException::new_err(e)),
        };
        let buffer = Arc::new(Mutex::new(PacketBuffer::new()));
        let closed = Arc::new(AtomicBool::new(false));
        let instance = RsVideoCapture {
            buffer: buffer.clone(),
            decoder: Mutex::new(decoder),
            closed: closed.clone(),
        };

        thread::spawn(move || {
            while !closed.load(Ordering::Relaxed) {
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
        });

        Ok(instance)
    }

    pub fn grab(&mut self) -> PyResult<Vec<u8>> {
        let mut decoder = self.decoder.lock().unwrap();
        let mut buffer = self.buffer.lock().unwrap();
        let frames = buffer.drain(..).flat_map(|packet| decoder.decode(&packet));
        match frames.last() {
            Some(frame) => Ok(frame),
            None => Err(PyException::new_err("No frame received")),
        }
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }

    pub fn width(&self) -> usize {
        self.decoder.lock().unwrap().width()
    }

    pub fn height(&self) -> usize {
        self.decoder.lock().unwrap().height()
    }
}

#[pymodule]
fn video_capture(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RsVideoCapture>()?;
    Ok(())
}
