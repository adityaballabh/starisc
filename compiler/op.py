from dataclasses import dataclass
from typing import Optional


@dataclass
class Op:
    opcode: str
    dst: str
    src1: Optional[str] = None
    src2: Optional[str] = None
