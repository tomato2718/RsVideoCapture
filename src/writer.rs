use crate::packet::Packet;
use rsmpeg::{
    avcodec::AVCodecParameters,
    avformat::AVFormatContextOutput,
    avutil::{AVDictionary, AVRational},
};
use std::ffi::CString;

pub struct VideoWriter {
    writer: AVFormatContextOutput,
    input_time_base: AVRational,
    output_time_base: AVRational,
    last_dts: i64,
}

impl VideoWriter {
    pub fn new(
        path: &str,
        codecpar: AVCodecParameters,
        input_time_base: AVRational,
    ) -> Result<Self, VideoWriterError> {
        Self::create_cpath(path)
            .and_then(Self::create_output)
            .and_then(|writer| Self::setup_stream(writer, codecpar))
            .map(|writer| {
                let output_time_base = writer.streams()[0].time_base;
                Self {
                    writer,
                    input_time_base,
                    output_time_base,
                    last_dts: 0,
                }
            })
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
        codecpar: AVCodecParameters,
    ) -> Result<AVFormatContextOutput, VideoWriterError> {
        {
            let mut stream = writer.new_stream();
            stream.set_codecpar(codecpar.clone());
        }
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
        packet.set_stream_index(0);
        packet.rescale_ts(self.input_time_base, self.output_time_base);

        if packet.dts <= self.last_dts {
            packet.set_dts(self.last_dts + 1);
        }
        self.last_dts = packet.dts;

        if packet.pts < packet.dts {
            packet.set_pts(packet.dts);
        }

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
