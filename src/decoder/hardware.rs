use rsmpeg::{
    avcodec::{AVCodecContext, AVCodecParametersRef, AVCodecRef},
    avutil::{AVFrame, AVHWDeviceContext, AVHWDeviceType},
    ffi::{AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX, AV_PIX_FMT_RGB24, SWS_BILINEAR},
    swscale::SwsContext,
};

use super::DecoderError;
use crate::types::Packet;

pub struct HardwareDecoder {
    decoder: AVCodecContext,
    scaler: Option<SwsContext>,
    frame_buffer: AVFrame,
    buffer_size: usize,
}

impl HardwareDecoder {
    pub fn new(
        codec: AVCodecRef,
        codecpar: AVCodecParametersRef,
        device_type: AVHWDeviceType,
    ) -> Result<Self, DecoderError> {
        Self::find_hw_config(&codec, device_type)?;
        let hw_device_ctx = AVHWDeviceContext::create(device_type, None, None, 0)
            .map_err(|_| DecoderError::FailedToOpenDecoder)?;
        let decoder = Self::create_decoder(codec, codecpar, hw_device_ctx)?;
        let frame_buffer = Self::create_frame_buffer(&decoder);
        let buffer_size = (3 * decoder.width * decoder.height) as usize;

        Ok(HardwareDecoder {
            decoder,
            scaler: None,
            frame_buffer,
            buffer_size,
        })
    }

    fn find_hw_config(codec: &AVCodecRef, device_type: AVHWDeviceType) -> Result<(), DecoderError> {
        (0..)
            .map_while(|i| codec.hw_config(i))
            .any(|config| {
                config.device_type == device_type
                    && (config.methods as u32 & AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX) != 0
            })
            .then_some(())
            .ok_or(DecoderError::NoHwConfig)
    }

    fn create_decoder(
        codec: AVCodecRef,
        codecpar: AVCodecParametersRef,
        hw_device_ctx: AVHWDeviceContext,
    ) -> Result<AVCodecContext, DecoderError> {
        let mut decoder = AVCodecContext::new(&codec);
        decoder.apply_codecpar(&codecpar).unwrap();
        decoder.set_hw_device_ctx(hw_device_ctx);
        match decoder.open(None) {
            Ok(_) => Ok(decoder),
            Err(_) => Err(DecoderError::FailedToOpenDecoder),
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
        if self.decoder.send_packet(Some(packet)).is_err() {
            return Vec::new();
        }
        let mut res = Vec::new();
        while let Ok(hw_frame) = self.decoder.receive_frame() {
            let mut sw_frame = AVFrame::new();
            if sw_frame.hwframe_transfer_data(&hw_frame).is_err() {
                continue;
            }

            if self.scaler.is_none() {
                self.scaler = Some(
                    SwsContext::get_context(
                        sw_frame.width,
                        sw_frame.height,
                        sw_frame.format,
                        sw_frame.width,
                        sw_frame.height,
                        AV_PIX_FMT_RGB24,
                        SWS_BILINEAR,
                        None,
                        None,
                        None,
                    )
                    .unwrap(),
                );
            };
            self.scaler
                .as_mut()
                .unwrap()
                .scale_frame(&sw_frame, 0, sw_frame.height, &mut self.frame_buffer)
                .unwrap();

            let mut buffer = vec![0u8; self.buffer_size];
            self.frame_buffer
                .image_copy_to_buffer(&mut buffer, 1)
                .expect("Should be ok");
            res.push(buffer);
        }
        res
    }
}
