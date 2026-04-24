# video-capture

## IMPORTANT

This repository has been moved to a new location and is no longer actively maintained.

Please use the new repository for the latest code, documentation, and issue tracking: [ez-video-capture](https://github.com/tomato2718/ez-video-capture).

---

An experimental Python library for capturing frames from video streams, written in Rust on top of FFmpeg via [`rsmpeg`](https://github.com/larksuite/rsmpeg) and exposed to Python through [PyO3](https://pyo3.rs/).

## Requirements

- Python ≥ 3.8
- FFmpeg development libraries and `pkg-config`

## API

```python
class RsVideoCapture:
    def __init__(
        self,
        path: str,
        *,
        timeout: int = 10000,
        hardware_acceleration: HardwareType | None = None,
    ) -> None: ...
    def grab(self) -> Option[bytes]: ...
    def width(self) -> int: ...
    def height(self) -> int: ...
    def close(self) -> None: ...

class HardwareType(Enum):
    VAAPI = 0        # Linux (Intel/AMD)
    VideoToolbox = 1 # macOS
    D3D11VA = 2      # Windows
    D3D12VA = 3      # Windows
    CUDA = 4         # Linux/Windows (NVIDIA)
    Vulkan = 5       # Linux/Windows
```

Pass `hardware_acceleration=None` (the default) to use the software decoder. Selecting a `HardwareType` not supported on the current OS raises `ValueError`.

## Example

Capture one frame per second from an RTSP stream and save each as a PNG:

```python
from time import sleep

from PIL import Image
from video_capture import RsVideoCapture, HardwareType

PATH = "rtsp://192.168.123.123:1234/"

capture = RsVideoCapture(PATH, hardware_acceleration=HardwareType.VAAPI)
width, height = capture.width(), capture.height()

for i in range(20):
    frame = capture.grab()
    if frame is not None:
        Image.frombytes(mode="RGB", size=(width, height), data=frame).save(
            f"frame_{i:03d}.png", format="PNG"
        )
    sleep(1)

capture.close()
```

`grab()` returns a `bytes` object containing the raw RGB24 pixel data for the latest frame (size `width * height * 3`), or `None` if no frame is available yet. Pass it to PIL, NumPy, OpenCV, or any library that can consume raw RGB buffers. Raises `ConnectionError` if the connection is closed.

## Exceptions

`RsVideoCapture(...)` may raise:

- `ConnectionError` — failed to open the video source, or no video stream found in source
- `IOError` — failure while reading from the source
- `ValueError` — failed to open the decoder, no matching hardware config for the codec, or selected `HardwareType` not supported on this platform

`grab()` may raise:

- `ConnectionError` — the connection has been closed
