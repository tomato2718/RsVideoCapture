__all__ = ["RsVideoCapture", "HardwareType"]

from enum import Enum
from typing import Optional

class RsVideoCapture:
    def __init__(
        self,
        path: str,
        /,
        *,
        timeout: int = 10000,
        hardware_acceleration: Optional["HardwareType"] = None,
        save_path: Optional[str] = None,
    ) -> None: ...
    def grab(self) -> Optional[bytes]: ...
    def close(self) -> None: ...
    def width(self) -> int: ...
    def height(self) -> int: ...

class HardwareType(Enum):
    VAAPI = 0
    VideoToolbox = 1
    D3D11VA = 2
    D3D12VA = 3
    CUDA = 4
    Vulkan = 5
