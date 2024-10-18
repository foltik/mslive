from dataclasses import dataclass
from enum import Enum
import random
from typing import Dict, List, NewType

# UnixTime = NewType('UnixTime', float)
DMXState = Dict[int, int]

@dataclass
class DMXEvent:
    time: float
    state: DMXState


class ScriptableDMX:
    state: DMXState
    script: List[DMXEvent]
    offset: int

    def __init__(self, offset) -> None:
        self.state = {}
        self.script = []
        self.offset = offset

    def emit(self, channel: int, value: int):
        # print('emit', self.state)
        self.state[channel+self.offset] = value

    def at(self, time: float):
        self.script.append(DMXEvent(time, self.state.copy()))
        # print('script', self.script)

    def reset(self) -> List[DMXEvent]:
        out = self.script[:]
        # self.script = []
        return out


class DiscoEnable(Enum):
    NONE = 0
    WHITE = 1
    UV = 2
    COLOR = 3
    COLOR_WHITE = 4
    COLOR_UV = 5

def generic():
    disco = ()
    beats = []
    for i, beat in enumerate(beats):
        match i % 4:
            case 0:
                disco.enable(DiscoEnable.WHITE)
                disco.emit(beat)
                disco.enable(DiscoEnable.NONE)
                disco.emit(beat+0.1)
            case 1:
                disco.enable(DiscoEnable.COLOR)
                disco.emit(beat)
            case 2:
                disco.enable(DiscoEnable.NONE)
                disco.emit(beat)
            case 3:
                disco.enable(DiscoEnable.COLOR)
                disco.emit(beat)


