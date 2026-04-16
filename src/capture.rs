use crate::types::Packet;
use rsmpeg::{
    avcodec::{AVCodecParametersRef, AVCodecRef},
    avformat::AVFormatContextInput,
    avutil::AVDictionary,
    ffi::AVMEDIA_TYPE_VIDEO,
};
use std::ffi::CString;

pub struct VideoCapture {
    input: AVFormatContextInput,
    video_index: i32,
}

impl VideoCapture {
    pub fn new(path: &str, timeout: u32) -> Result<(Self, AVCodecRef<'_>), VideoCaptureError> {
        let input = Self::create_cpath(path).and_then(|path| Self::create_input(path, timeout))?;
        let (video_index, codec) = match input.find_best_stream(AVMEDIA_TYPE_VIDEO) {
            Ok(Some((index, codec))) => Ok((index as i32, codec)),
            _ => Err(VideoCaptureError::FailedToFindVideo),
        }?;
        let capture = VideoCapture { input, video_index };
        if capture.codecpar().width <= 0 {
            Err(VideoCaptureError::FailedToOpenFile)
        } else {
            Ok((capture, codec))
        }
    }

    fn create_cpath(path: &str) -> Result<CString, VideoCaptureError> {
        CString::new(path).map_err(|_| VideoCaptureError::FailedToOpenFile)
    }

    fn create_input(
        cpath: CString,
        timeout: u32,
    ) -> Result<AVFormatContextInput, VideoCaptureError> {
        let timeout_us = CString::new((timeout as u64 * 1000).to_string())
            .map_err(|_| VideoCaptureError::FailedToOpenFile)?;
        AVFormatContextInput::builder()
            .url(cpath.as_ref())
            .options(&mut Some(
                AVDictionary::new(c"rtsp_transport", c"tcp", 0).set(c"timeout", &timeout_us, 0),
            ))
            .open()
            .map_err(|_| VideoCaptureError::FailedToOpenFile)
    }

    pub fn receive(&mut self) -> Result<Option<Packet>, VideoCaptureError> {
        loop {
            let packet = match self.input.read_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return Ok(None),
                Err(_) => return Err(VideoCaptureError::ErrReadingFile),
            };
            if packet.stream_index != self.video_index {
                continue;
            }
            return Ok(Some(packet));
        }
    }

    pub fn codecpar(&self) -> AVCodecParametersRef<'_> {
        self.input.streams()[self.video_index as usize].codecpar()
    }
}

pub enum VideoCaptureError {
    FailedToOpenFile,
    FailedToFindVideo,
    ErrReadingFile,
}
