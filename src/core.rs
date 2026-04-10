use rsmpeg::{
    avcodec::{AVCodecContext, AVPacket},
    avformat::AVFormatContextInput,
    avutil::AVFrame,
    ffi::{AVMEDIA_TYPE_VIDEO, AV_PIX_FMT_RGB24, SWS_BILINEAR},
    swscale::SwsContext,
};
use std::ffi::CString;

type ErrorMessage = &'static str;
const ERR_OPEN_FILE: ErrorMessage = "Failed to open video";
const ERR_FAILED_TO_FIND_VIDEO: ErrorMessage = "Failed to find video stream";
const ERR_FAILED_TO_OPEN_DECODER: ErrorMessage = "Failed to open decoder";
const ERR_READING_FILE: ErrorMessage = "Error reading file";

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
            return Ok(Some(packet));
        }
    }
}

pub struct VideoDecoder {
    decoder: AVCodecContext,
    scaler: SwsContext,
    rgb_frame: AVFrame,
}

impl VideoDecoder {
    pub fn new(decoder: AVCodecContext, scaler: SwsContext) -> Self {
        let mut rgb_frame = AVFrame::new();
        rgb_frame.set_width(decoder.width);
        rgb_frame.set_height(decoder.height);
        rgb_frame.set_format(AV_PIX_FMT_RGB24);
        rgb_frame.get_buffer(1).unwrap();
        VideoDecoder {
            decoder,
            scaler,
            rgb_frame,
        }
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
            self.scaler
                .scale_frame(&frame, 0, frame.height, &mut self.rgb_frame)
                .unwrap();

            let mut buffer = vec![0u8; self.rgb_frame.image_get_buffer_size(1).unwrap()];
            self.rgb_frame
                .image_copy_to_buffer(&mut buffer, 1)
                .expect("Should be ok");
            res.push(buffer)
        }
        res
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
            Some(sws) => Ok((input, index, decoder, sws)),
            None => Err(ERR_FAILED_TO_OPEN_DECODER),
        }
    })
    .map(|(input, index, decoder, sws)| {
        (
            VideoCapture::new(input, index as i32),
            VideoDecoder::new(decoder, sws),
        )
    })
}
