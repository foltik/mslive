from dataclasses import dataclass
from typing import List

@dataclass
class Bar:
    beats: List[float]
    start: float
    half: float
    end: float

@dataclass
class Section:
    bars: List[Bar]
    start: float
    end: float
