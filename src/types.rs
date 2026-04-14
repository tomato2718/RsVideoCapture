use rsmpeg::avcodec::AVPacket;

pub type VideoCaptureError = &'static str;
pub type Packet = AVPacket;
