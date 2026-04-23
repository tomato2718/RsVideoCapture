use crate::packet::Packet;
use rsmpeg::avformat::AVFormatContextOutput;
use rsmpeg::avutil::AVDictionary;
use std::ffi::CString;

pub struct VideoWriter {
    writer: AVFormatContextOutput,
}

impl VideoWriter {
    pub fn new(path: &str) -> Result<Self, VideoWriterError> {
        Self::create_cpath(path)
            .and_then(Self::create_output)
            .and_then(Self::setup_stream)
            .map(|writer| Self { writer })
    }

    fn create_cpath(path: &str) -> Result<CString, VideoWriterError> {
        CString::new(path).map_err(|_| VideoWriterError::FailedToOpenFile)
    }

    fn create_output(path: CString) -> Result<AVFormatContextOutput, VideoWriterError> {
        AVFormatContextOutput::builder()
            .format_name(c"mp4")
            .filename(&path)
            .build()
            .map_err(|_| VideoWriterError::FailedToOpenFile)
    }

    fn setup_stream(
        mut writer: AVFormatContextOutput,
    ) -> Result<AVFormatContextOutput, VideoWriterError> {
        writer.new_stream();
        match writer.write_header(&mut Some(AVDictionary::new(
            c"movflags",
            c"frag_keyframe+empty_moov+default_base_moof",
            0,
        ))) {
            Ok(_) => Ok(writer),
            Err(_) => Err(VideoWriterError::FailedToOpenFile),
        }
    }

    pub fn push(&mut self, mut packet: Packet) -> Result<(), VideoWriterError> {
        self.writer
            .write_frame(&mut packet)
            .map_err(|_| VideoWriterError::FailedToWriteFrame)
    }

    pub fn end(&mut self) -> Result<(), VideoWriterError> {
        self.writer
            .write_trailer()
            .map_err(|_| VideoWriterError::FailedToWriteTrailer)
    }
}

#[allow(clippy::enum_variant_names)]
pub enum VideoWriterError {
    FailedToOpenFile,
    FailedToWriteFrame,
    FailedToWriteTrailer,
}
