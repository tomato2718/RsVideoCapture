use rsmpeg::{
    avcodec::{AVCodecContext, AVCodecParametersRef, AVCodecRef},
    avutil::AVFrame,
    ffi::{AV_PIX_FMT_RGB24, SWS_BILINEAR},
    swscale::SwsContext,
};

use super::DecoderError;
use crate::types::Packet;

pub struct SoftwareDecoder {
    decoder: AVCodecContext,
    scaler: SwsContext,
    frame_buffer: AVFrame,
    buffer_size: usize,
}

impl SoftwareDecoder {
    pub fn new(codec: AVCodecRef, codecpar: AVCodecParametersRef) -> Result<Self, DecoderError> {
        let decoder = Self::create_decoder(codec, codecpar)?;
        let scaler = Self::create_scaler(&decoder)?;
        let frame_buffer = Self::create_frame_buffer(&decoder);
        let buffer_size = (3 * decoder.width * decoder.height) as usize;

        Ok(SoftwareDecoder {
            decoder,
            scaler,
            frame_buffer,
            buffer_size,
        })
    }
    fn create_decoder(
        codec: AVCodecRef,
        codecpar: AVCodecParametersRef,
    ) -> Result<AVCodecContext, DecoderError> {
        let mut decoder = AVCodecContext::new(&codec);
        decoder.apply_codecpar(&codecpar).unwrap();
        match decoder.open(None) {
            Ok(_) => Ok(decoder),
            Err(_) => Err(DecoderError::FailedToOpenDecoder),
        }
    }

    fn create_scaler(decoder: &AVCodecContext) -> Result<SwsContext, DecoderError> {
        match SwsContext::get_context(
            decoder.width,
            decoder.height,
            decoder.pix_fmt,
            decoder.width,
            decoder.height,
            AV_PIX_FMT_RGB24,
            SWS_BILINEAR,
            None,
            None,
            None,
        ) {
            Some(sws) => Ok(sws),
            None => Err(DecoderError::FailedToOpenDecoder),
        }
    }

    fn create_frame_buffer(decoder: &AVCodecContext) -> AVFrame {
        let mut rgb_frame = AVFrame::new();
        rgb_frame.set_width(decoder.width);
        rgb_frame.set_height(decoder.height);
        rgb_frame.set_format(AV_PIX_FMT_RGB24);
        rgb_frame.get_buffer(1).unwrap();
        rgb_frame
    }

    pub fn width(&self) -> usize {
        self.decoder.width as usize
    }

    pub fn height(&self) -> usize {
        self.decoder.height as usize
    }

    pub fn decode(&mut self, packet: &Packet) -> Vec<Vec<u8>> {
        self.decoder
            .send_packet(Some(packet))
            .expect("Should be ok");
        let mut res = Vec::new();
        while let Ok(frame) = self.decoder.receive_frame() {
            self.scaler
                .scale_frame(&frame, 0, frame.height, &mut self.frame_buffer)
                .unwrap();

            let mut buffer = vec![0u8; self.buffer_size];
            self.frame_buffer
                .image_copy_to_buffer(&mut buffer, 1)
                .expect("Should be ok");
            res.push(buffer)
        }
        res
    }
}
