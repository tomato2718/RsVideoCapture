use rsmpeg::{
    avcodec::{AVCodecParametersRef, AVCodecRef},
    avformat::AVFormatContextInput,
    ffi::AVMEDIA_TYPE_VIDEO,
};
use std::ffi::CString;

use crate::types::{Packet, VideoCaptureError};

const ERR_OPEN_FILE: VideoCaptureError = "Failed to open video";
const ERR_FAILED_TO_FIND_VIDEO: VideoCaptureError = "Failed to find video stream";
const ERR_READING_FILE: VideoCaptureError = "Error reading file";

pub struct VideoCapture {
    input: AVFormatContextInput,
    video_index: i32,
}

impl VideoCapture {
    pub fn new(path: &str) -> Result<(Self, AVCodecRef<'_>), VideoCaptureError> {
        let input = Self::create_cpath(path).and_then(Self::create_input)?;
        let (video_index, codec) = match input.find_best_stream(AVMEDIA_TYPE_VIDEO) {
            Ok(Some((index, codec))) => Ok((index as i32, codec)),
            _ => Err(ERR_FAILED_TO_FIND_VIDEO),
        }?;
        Ok((VideoCapture { input, video_index }, codec))
    }

    fn create_cpath(path: &str) -> Result<CString, VideoCaptureError> {
        CString::new(path).map_err(|_| ERR_OPEN_FILE)
    }

    fn create_input(cpath: CString) -> Result<AVFormatContextInput, VideoCaptureError> {
        AVFormatContextInput::open(cpath.as_ref()).map_err(|_| ERR_OPEN_FILE)
    }

    pub fn receive(&mut self) -> Result<Option<Packet>, VideoCaptureError> {
        loop {
            let packet = match self.input.read_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return Ok(None),
                Err(_) => return Err(ERR_READING_FILE),
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
