use rsmpeg::{
    avcodec::{AVCodecContext, AVPacket},
    avformat::AVFormatContextInput,
    ffi::AVMEDIA_TYPE_VIDEO,
};
use std::collections::VecDeque;
use std::ffi::CString;

type ErrorMessage = &'static str;
const ERR_OPEN_FILE: ErrorMessage = "Failed to open video";
const ERR_FAILED_TO_FIND_VIDEO: ErrorMessage = "Failed to find video stream";
const ERR_FAILED_TO_OPEN_DECODER: ErrorMessage = "Failed to open decoder";
const ERR_READING_FILE: ErrorMessage = "Error reading file";

pub type PacketBuffer = VecDeque<AVPacket>;
pub type Packet = AVPacket;

pub struct VideoCapture {
    input: AVFormatContextInput,
    video_index: i32,
}

impl VideoCapture {
    pub fn new(input: AVFormatContextInput, video_index: i32) -> Self {
        VideoCapture { input, video_index }
    }

    pub fn receive(&mut self) -> Result<Option<AVPacket>, ErrorMessage> {
        loop {
            let packet = match self.input.read_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return Ok(None),
                Err(_) => return Err(ERR_READING_FILE),
            };
            if packet.stream_index != self.video_index {
                continue;
            }
            return Ok(Some(AVPacket::from(packet)));
        }
    }
}

pub struct VideoDecoder {
    decoder: AVCodecContext,
}

impl VideoDecoder {
    pub fn new(decoder: AVCodecContext) -> Self {
        VideoDecoder { decoder }
    }

    pub fn width(&self) -> usize {
        self.decoder.width as usize
    }

    pub fn height(&self) -> usize {
        self.decoder.height as usize
    }

    pub fn decode(&mut self, packet: &AVPacket) -> Vec<Vec<u8>> {
        self.decoder
            .send_packet(Some(packet))
            .expect("Should be ok");
        let mut res = Vec::new();
        while let Ok(frame) = self.decoder.receive_frame() {
            let mut buffer = vec![0u8; frame.image_get_buffer_size(1).unwrap()];
            frame
                .image_copy_to_buffer(&mut buffer, 1)
                .expect("Should be ok");
            res.push(VideoDecoder::yuv_to_rgb(
                &buffer,
                frame.width as usize,
                frame.height as usize,
            ))
        }
        res
    }

    fn yuv_to_rgb(frame: &[u8], width: usize, height: usize) -> Vec<u8> {
        let y_size = width * height;
        let u_size = (width / 2) * (height / 2);
        let v_size = (width / 2) * (height / 2);

        let y_plane = &frame[0..y_size];
        let u_plane = &frame[y_size..y_size + u_size];
        let v_plane = &frame[y_size + u_size..y_size + u_size + v_size];

        let mut rgb = Vec::with_capacity(width * height * 3);

        for y in 0..height {
            for x in 0..width {
                let y_idx = y * width + x;
                let uv_idx = (y / 2) * (width / 2) + (x / 2);

                let y_val = y_plane[y_idx] as i32;
                let u_val = u_plane[uv_idx] as i32;
                let v_val = v_plane[uv_idx] as i32;

                let c = y_val - 16;
                let d = u_val - 128;
                let e = v_val - 128;

                let r = ((298 * c + 409 * e + 128) >> 8).clamp(0, 255) as u8;
                let g = ((298 * c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8;
                let b = ((298 * c + 516 * d + 128) >> 8).clamp(0, 255) as u8;

                rgb.push(r);
                rgb.push(g);
                rgb.push(b);
            }
        }

        rgb
    }
}

pub fn connect(path: &str) -> Result<(VideoCapture, VideoDecoder), ErrorMessage> {
    match CString::new(path) {
        Ok(cpath) => Ok(cpath),
        Err(_) => Err(ERR_OPEN_FILE),
    }
    .and_then(|cpath| match AVFormatContextInput::open(&cpath) {
        Ok(context) => Ok(context),
        Err(_) => Err(ERR_OPEN_FILE),
    })
    .and_then(|input| match input.find_best_stream(AVMEDIA_TYPE_VIDEO) {
        Ok(Some((index, codec))) => Ok((input, index, codec)),
        _ => Err(ERR_FAILED_TO_FIND_VIDEO),
    })
    .and_then(|(input, index, codec)| {
        let mut decoder = AVCodecContext::new(&codec);
        decoder
            .apply_codecpar(&input.streams()[index].codecpar())
            .unwrap();
        match decoder.open(None) {
            Ok(_) => Ok((input, index, decoder)),
            _ => Err(ERR_FAILED_TO_OPEN_DECODER),
        }
    })
    .and_then(|(input, index, decoder)| {
        Ok((
            VideoCapture::new(input, index as i32),
            VideoDecoder::new(decoder),
        ))
    })
}
