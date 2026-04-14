# video-capture

An experimental Python library for capturing frames from video streams, written in Rust on top of FFmpeg via [`rsmpeg`](https://github.com/larksuite/rsmpeg) and exposed to Python through [PyO3](https://pyo3.rs/).

## Requirements

- Python ≥ 3.8
- FFmpeg development libraries and `pkg-config`

## API

```python
class RsVideoCapture:
    def __init__(self, path: str, *, use_hardware: bool) -> None: ...
    def grab(self) -> bytes: ...
    def width(self) -> int: ...
    def height(self) -> int: ...
    def close(self) -> None: ...
```

## Example

Capture one frame per second from an RTSP stream and save each as a PNG:

```python
from time import sleep

from PIL import Image
from video_capture import RsVideoCapture

PATH = "rtsp://192.168.123.123:1234/"

capture = RsVideoCapture(PATH, use_hardware=False)
width, height = capture.width(), capture.height()

for i in range(20):
    try:
        frame = capture.grab()
        Image.frombytes(mode="RGB", size=(width, height), data=frame).save(
            f"frame_{i:03d}.png", format="PNG"
        )
    except Exception as e:
        print(e)
    sleep(1)

capture.close()
```

`grab()` returns a `bytes` object containing the raw RGB24 pixel data for the latest frame (size `width * height * 3`). Pass it to PIL, NumPy, OpenCV, or any library that can consume raw RGB buffers.
