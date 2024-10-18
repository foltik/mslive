from curses import echo
from dataclasses import dataclass
from enum import Enum, auto
import time
from typing import List, Tuple

import dmx

FORCE_UV = True

class SpinColors(Enum):
    NONE = auto()
    RED = auto()
    YELLOW = auto()
    BLUE = auto()
    RED_YELLOW = auto()
    RED_BLUE = auto()
    YELLOW_BLUE = auto()
    ALL = auto()


class SpinEnable(Enum):
    NONE = auto()
    UV = auto()
    WHITE = auto()
    COLOR = auto()
    COLOR_UV = auto()
    COLOR_WHITE = auto()


class OneSpinLight(dmx.ScriptableDMX):
    def enable(self, mode: SpinEnable):
        if mode == SpinEnable.COLOR and FORCE_UV:
            mode = SpinEnable.COLOR_UV
            self.uv(True)

        match mode:
            case SpinEnable.NONE:
                self.emit(4, 0)
            case SpinEnable.UV:
                self.emit(4, 40)
            case SpinEnable.WHITE:
                self.emit(4, 80)
            case SpinEnable.COLOR:
                self.emit(4, 120)
            case SpinEnable.COLOR_UV:
                self.emit(4, 190)
            case SpinEnable.COLOR_WHITE:
                self.emit(4, 230)
            case _:
                raise NotImplementedError()

    def color(self, color: SpinColors):
        match color:
            case SpinColors.NONE:
                self.emit(3, 0)
            case SpinColors.RED:
                self.emit(3, 20)
            case SpinColors.YELLOW:
                self.emit(3, 50)
            case SpinColors.BLUE:
                self.emit(3, 90)
            case SpinColors.RED_YELLOW:
                self.emit(3, 120)
            case SpinColors.RED_BLUE:
                self.emit(3, 180)
            case SpinColors.YELLOW_BLUE:
                self.emit(3, 200)
            case SpinColors.ALL:
                self.emit(3, 220)
            case _:
                raise NotImplementedError()

    def white(self, on: bool):
        if on:
            self.emit(2, 50)
        else:
            self.emit(2, 0)

    def uv(self, on: bool):
        if on or FORCE_UV:
            self.emit(1, 50)
        else:
            self.emit(1, 0)


    def strobe(self, speed: int):
        self.emit(5, speed)

    def rotate(self, speed: int):
        self.emit(6, speed)


class TwoSpinLights(OneSpinLight):
    left: OneSpinLight
    right: OneSpinLight

    def __init__(self, offset_left, offset_right):
        self.left = OneSpinLight(offset_left)
        self.right = OneSpinLight(offset_right)

    def emit(self, channel: int, value: int):
        # print('ch', channel, 'v', value)
        self.left.emit(channel, value)
        self.right.emit(channel, value)

    def at(self, time: float):
        self.left.at(time)
        self.right.at(time)

    def reset(self) -> List[dmx.DMXEvent]:
        tmp = self.left.reset() + self.right.reset()
        tmp.sort(key=lambda e: e.time)
        if len(tmp) == 0:
            return []
        out = tmp[:1]
        for e in tmp[1:]:
            if e.time == out[-1].time:
                out[-1].state |= e.state
            else:
                out.append(e)
        return out

@dataclass
class JackEvent:
    time: float
    state: str

class Jack:
    state: str
    script: List[JackEvent]

    def __init__(self) -> None:
        self.state = {}
        self.script = []


    def at(self, time: float, state: str):
        self.script.append(JackEvent(time, state))
        # print('script', self.script)

    def reset(self):
        out = self.script[:]
        self.script = []
        return out