mod binding;
mod capture;
mod decoder;
mod types;

fn connect(
    path: &str,
    timeout: u32,
    use_hardware: bool,
) -> Result<(capture::VideoCapture, decoder::VideoDecoder), types::VideoCaptureError> {
    let (capture, codec) = capture::VideoCapture::new(path, timeout)?;
    let decoder = if use_hardware {
        decoder::VideoDecoder::new_hardware(codec, capture.codecpar())?
    } else {
        decoder::VideoDecoder::new_software(codec, capture.codecpar())?
    };
    Ok((capture, decoder))
}
