use std::ops::{Deref, DerefMut};

use async_trait::async_trait;

use stagebridge::midi::device::launch_control_xl::types::{
    Brightness as CtrlBrightness, Color as CtrlColor,
};
use stagebridge::midi::device::launch_control_xl::{
    self, Input as CtrlInput, LaunchControlXL, Output as CtrlOutput,
};
use stagebridge::midi::device::launchpad_x::types::{Color as PadColor, Coord, PaletteColor, Pos};
use stagebridge::midi::device::launchpad_x::{
    self, Input as PadInput, LaunchpadX, Output as PadOutput,
};
use stagebridge::midi::Midi;
use stagebridge::num::Float;

type Pad = Midi<LaunchpadX>;
type Ctrl = Midi<LaunchControlXL>;

use crate::color::Color;
use crate::{lights::*, Pd};
use crate::{ClockSource, State};

use crate::fx::{self, ColorFn, ColorOp, LightColorFn, LightColorOp, LightFn, LightOp};

use super::Logic;

pub struct Beams {
    pub pattern: BeamPattern,
    pub color: BeamColor,
    pub ring: BeamRing,
}
pub enum BeamColor {
    Color0,
    Color1,
    Alternate,
    Roll { pd: Pd, duty: f64, offset: f64, alpha: f64 },
}
pub enum BeamPattern {
    Down,
    Out,
    SpreadOut,
    SpreadIn,
    Cross,
    CrissCross,
    WaveY { pd: Pd },
    SnapX { pd: Pd },
    SnapY { pd: Pd },
    Square { pd: Pd },
}
#[async_trait]
impl Logic for Beams {
    // fn pad(&mut self, _state: &mut State, _input: PadInput) {}

    // fn ctrl(&mut self, _: &mut State, input: CtrlInput) {
    //     match input {
    //         CtrlInput::Control(i, true) => self.pattern = match i {
    //             1 => BeamPattern::SpreadOut,
    //             2 => BeamPattern::SpreadIn,
    //             3 => BeamPattern::Cross,
    //             4 => BeamPattern::CrissCross,
    //             5 => BeamPattern::WaveX { pd: Pd(2, 1) },
    //             6 => BeamPattern::WaveY { pd: Pd(2, 1) },
    //             7 => BeamPattern::Square { pd: Pd(2, 1) },
    //             _ => BeamPattern::Down,
    //         },
    //         _ => {},
    //     }
    // }

    async fn output(&self, state: &State, lights: &mut Lights, _: &Pad, _: &Ctrl) {
        let mut beams = [Beam::default(); 4];

        match self.pattern {
            BeamPattern::Down => {
                for beam in &mut beams {
                    beam.pitch = 0.0;
                    beam.yaw = 0.5;
                }
            },
            BeamPattern::Out => {
                for beam in &mut beams {
                    beam.pitch = 0.5;
                    beam.yaw = 0.5;
                }
            },
            BeamPattern::SpreadOut => {
                beams[0].yaw = 0.5 - 0.05;
                beams[1].yaw = 0.5 - 0.02;
                beams[2].yaw = 0.5 + 0.02;
                beams[3].yaw = 0.5 + 0.05;
            },
            BeamPattern::SpreadIn => {
                beams[0].yaw = 0.5 + 0.09;
                beams[1].yaw = 0.5 + 0.07;
                beams[2].yaw = 0.5 - 0.07;
                beams[3].yaw = 0.5 - 0.09;
            },
            BeamPattern::Cross => {
                beams[0].yaw = 0.5 + 0.13;
                beams[1].yaw = 0.5 + 0.13;
                beams[2].yaw = 0.5 - 0.13;
                beams[3].yaw = 0.5 - 0.13;
            },
            BeamPattern::CrissCross => {
                beams[0].yaw = 0.5 + 0.08;
                beams[1].yaw = 0.5 - 0.05;
                beams[2].yaw = 0.5 + 0.05;
                beams[3].yaw = 0.5 - 0.08;
            },
            BeamPattern::SnapY { pd } => {
                for (i, beam) in beams.iter_mut().enumerate() {
                    beam.yaw = 0.5;
                    let t = state.phi(pd.mul(4)).square(1.0, 0.5);
                    beam.pitch = 0.3 * match i % 2 == 0 {
                        true => t,
                        false => 1.0 - t,
                    }
                }
            }
            BeamPattern::SnapX { pd } => {
                for (i, beam) in beams.iter_mut().enumerate() {
                    let t = state.phi(pd.mul(4)).negsquare(1.0, 0.5);
                    beam.yaw = 0.5 + 0.13 * match i > 1 {
                        true => t,
                        false => -t,
                    };
                    beam.pitch = 0.3 * state.phi(pd.mul(2)).square(1.0, 0.5);
                }
            },
            BeamPattern::WaveY { pd } => {
                for (i, beam) in beams.iter_mut().enumerate() {
                    beam.yaw = 0.5;
                    let t = state.phi(pd.mul(2)).tri(1.0);
                    beam.pitch = 0.3 * match i % 2 == 0 {
                        true => t,
                        false => 1.0 - t,
                    }
                }
            }
            BeamPattern::Square { pd } => {
                for (i, beam) in beams.iter_mut().enumerate() {
                    let t_pitch = state.phi(pd.mul(4)).phase(1.0, 0.25).square(1.0, 0.5);
                    let t_yaw = match i % 2 == 0 {
                        true => state.phi(pd.mul(4)).negsquare(1.0, 0.5),
                        false => state.phi(pd.mul(4)).phase(1.0, 0.5).negsquare(1.0, 0.5)
                    };
                    beam.pitch = 0.1 + 0.25 * match i % 2 == 0 {
                        true => t_pitch,
                        false => 1.0 - t_pitch,
                    };
                    beam.yaw = 0.5 + 0.08 * t_yaw;
                }
            }
        }

        for (i, beam) in beams.iter_mut().enumerate() {
            beam.ring = self.ring;
            beam.color = match self.color {
                BeamColor::Color0 => state.color0(),
                BeamColor::Color1 => state.color1(),
                BeamColor::Alternate => match i % 2 == 0 {
                    true => state.color0(),
                    false => state.color1(),
                },
                BeamColor::Roll { pd, duty, offset, alpha } => {
                    let t = state.phi(pd);
                    let a = t.phase(1.0, offset + 0.25 * i as f64).square(1.0, duty);
                    state.color1().a(a * alpha)
                }
            };
        }

        lights.beams = beams;
    }
}
impl Beams {
    pub fn new() -> Self {
        Self {
            pattern: BeamPattern::Down,
            color: BeamColor::Color0,
            ring: BeamRing::Off,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}



pub struct Lasers {
    laser: Laser,
    pub pos: LaserPos,
}
#[derive(Clone, Copy, Debug)]
pub enum LaserPos {
    Still,
    Rotate { pd: Pd },
    WaveY { pd: Pd },
}
#[async_trait]
impl Logic for Lasers {
    // fn pad(&mut self,_state: &mut State,_input:PadInput){}

    // fn ctrl(&mut self, _: &mut State, input: CtrlInput) {
    //     match input {
    //         CtrlInput::Focus(0, true) => self.laser.active = !self.laser.active,
    //         CtrlInput::Slider(0, fr) => self.laser.pattern = LaserPattern::Raw(fr.byte()),
    //         CtrlInput::Slider(1, fr) => self.laser.rotate = fr,
    //         CtrlInput::Slider(2, fr) => self.laser.xflip = fr,
    //         CtrlInput::Slider(3, fr) => self.laser.yflip = fr,
    //         CtrlInput::Slider(4, fr) => self.laser.x = fr,
    //         CtrlInput::Slider(5, fr) => self.laser.y = fr,
    //         CtrlInput::Slider(6, fr) => self.laser.size = fr,
    //         CtrlInput::Slider(7, fr) => self.laser.color = LaserColor::Raw(fr.byte()),
    //         _ => {}
    //     }
    // }

    async fn output(&self, state: &State, lights: &mut Lights, _: &Pad, _: &Ctrl) {
        let mut laser = self.laser.clone();

        match self.pos {
            LaserPos::Rotate { pd } => laser.rotate = state.phi(pd),
            LaserPos::WaveY { pd } => laser.y = state.phi(pd),
            LaserPos::Still => {},
        }

        lights.laser = laser;

        // ctrl.send(CtrlOutput::Focus(
        //     0,
        //     CtrlColor::Red,
        //     match self.laser.active {
        //         true => CtrlBrightness::High,
        //         false => CtrlBrightness::Off,
        //     },
        // ))
        // .await;
    }

}
impl Lasers {
    pub fn new() -> Self {
        Self {
            laser: Laser::default(),
            pos: LaserPos::Still,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
impl Deref for Lasers {
    type Target = Laser;

    fn deref(&self) -> &Self::Target {
        &self.laser
    }
}
impl DerefMut for Lasers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.laser
    }
}

pub struct Pars {
    pub color: ParColor,
}
#[derive(Clone, Copy, Debug)]
pub enum ParColor {
    Off,
    Color0,
    Color1,
    Alternate,
    UpDown,
    Spotlight,
    Roll { pd: Pd, ofs: f64 },
    StrobeAlt0 { pd: Pd, duty: f64 },
    StrobeAlt1 { pd: Pd, duty: f64 },
    StrobeRoll0 { pd: Pd, duty: f64, offset: f64 },
    StrobeRoll1 { pd: Pd, duty: f64, offset: f64 },
}
#[async_trait]
impl Logic for Pars {
    async fn output(&self, state: &State, lights: &mut Lights, _: &Pad, _: &Ctrl) {
        let color0 = state.color0();
        let color1 = state.color1();
        for (i, par) in lights.pars.iter_mut().enumerate() {
            par.color = match self.color {
                ParColor::Off => Color::OFF,
                ParColor::Color0 => color0,
                ParColor::Color1 => color1,
                ParColor::Alternate => match i % 2 == 0 {
                    true => color0,
                    false => color1,
                },
                ParColor::UpDown => match i {
                    0 => color1,
                    1 => color0,
                    2..=3 => color1,
                    4..=5 => color0,
                    6..=7 => color1,
                    8 => color0,
                    9 => color1,
                    _ => unreachable!(),
                },
                ParColor::Spotlight => match i {
                    3 => color1,
                    6 => color1,
                    _ => color0,
                },
                ParColor::Roll { pd, ofs } => {
                    let i = match state.phi(pd.mul(2)).bsquare(1.0, 0.5) {
                        true => i,
                        false => 10 - i,
                    };
                    state.color0_phase(pd, ofs + 0.1 * i as f64)
                },
                ParColor::StrobeAlt0 { pd, duty } => {
                    let t = state.phi(pd);
                    let offset = if i > 4 { 0.5 } else { 0.0 };
                    let a = t.phase(1.0, offset).square(1.0, duty);
                    state.color0().a(a)
                },
                ParColor::StrobeAlt1 { pd, duty } => {
                    let t = state.phi(pd);
                    let offset = if i > 4 { 0.5 } else { 0.0 };
                    let a = t.phase(1.0, offset).square(1.0, duty);
                    state.color1().a(a)
                },
                ParColor::StrobeRoll0 { pd, duty, offset } => {
                    let t = state.phi(pd);
                    let a = t.phase(1.0, offset + 0.1 * i as f64).square(1.0, duty);
                    state.color0().a(a)
                },
                ParColor::StrobeRoll1 { pd, duty, offset } => {
                    let t = state.phi(pd);
                    let a = t.phase(1.0, offset + 0.1 * i as f64).square(1.0, duty);
                    state.color1().a(a)
                },
            };
        }
    }

}
impl Pars {
    pub fn new() -> Self {
        Self {
            color: ParColor::Color0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}


pub struct Bars {
    pub color: BarColor,
}
#[derive(Clone, Copy, Debug)]
pub enum BarColor {
    Off,
    Color0,
    Color1,
    Roll { pd: Pd, duty: f64, offset: f64 },
}
#[async_trait]
impl Logic for Bars {
    async fn output(&self, state: &State, lights: &mut Lights, _: &Pad, _: &Ctrl) {
        for (i, bar) in lights.bars.iter_mut().enumerate() {
            bar.color = match self.color {
                BarColor::Off => Color::OFF,
                BarColor::Color0 => state.color0(),
                BarColor::Color1 => state.color1(),
                BarColor::Roll { pd, duty, offset } => {
                    let t = state.phi(pd);
                    let a = t.phase(1.0, offset + 0.5 * i as f64).square(1.0, duty);
                    state.color1().a(a)
                }
            };
        }
    }
}
impl Bars {
    pub fn new() -> Self {
        Self {
            color: BarColor::Color0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

pub struct Spiders {
    pub color: SpiderColor,
    pub pattern: SpiderPattern,
}
#[derive(Clone, Copy, Debug)]
pub enum SpiderColor {
    Off,
    Color0,
    Color1,
    Both,
}
#[derive(Clone, Copy, Debug)]
pub enum SpiderPattern {
    Up,
    Down,
    Wave { pd: Pd },
    Alternate { pd: Pd },
    Snap { pd: Pd },
}
#[async_trait]
impl Logic for Spiders {
    async fn output(&self, state: &State, lights: &mut Lights, _: &Pad, _: &Ctrl) {
        let color0 = state.color0();
        let color1 = state.color1();
        for (i, spider) in lights.spiders.iter_mut().enumerate() {
            match self.color {
                SpiderColor::Off => {
                    spider.color0 = Color::OFF;
                    spider.color1 = Color::OFF;
                },
                SpiderColor::Color0 => {
                    spider.color0 = color0;
                    spider.color1 = color0;
                },
                SpiderColor::Color1 => {
                    spider.color0 = color1;
                    spider.color1 = color1;
                },
                SpiderColor::Both => {
                    spider.color0 = color0;
                    spider.color1 = color1;
                },
            }

            match self.pattern {
                SpiderPattern::Up => {
                    spider.pos0 = 0.0;
                    spider.pos1 = 0.52;
                },
                SpiderPattern::Down => {
                    spider.pos0 = 0.67;
                    spider.pos1 = 0.52;
                },
                SpiderPattern::Wave { pd } => {
                    let fr = state.phi(pd.mul(2)).tri(1.0);
                    spider.pos0 = fr;
                    spider.pos1 = 1.0 - fr;
                },
                SpiderPattern::Alternate { pd } => {
                    let t = state.phi(pd.mul(2));
                    let t = match i {
                        0 => t,
                        1 => t.phase(1.0, 0.5),
                        _ => unreachable!(),
                    };
                    let fr = t.tri(1.0);
                    spider.pos0 = fr;
                    spider.pos1 = fr;
                },
                SpiderPattern::Snap { pd } => {
                    let t = state.phi(pd.mul(2));
                    let t = match i {
                        0 => t,
                        1 => t.phase(1.0, 0.5),
                        _ => unreachable!(),
                    };
                    let fr = t.square(1.0, 0.5);
                    spider.pos0 = fr;
                    spider.pos1 = fr;
                },
            }
        }
    }
}
impl Spiders {
    pub fn new() -> Self {
        Self {
            color: SpiderColor::Color0,
            pattern: SpiderPattern::Down,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

pub struct Strobes {
    pub color: StrobeColor,
}
#[derive(Clone, Copy, Debug)]
pub enum StrobeColor {
    Off,
    Color0,
    Color1,
    Strobe { pd: Pd, duty: f64, alpha: f64 },
}
#[async_trait]
impl Logic for Strobes {
    async fn output(&self, state: &State, lights: &mut Lights, _: &Pad, _: &Ctrl) {
        lights.strobe.color = match self.color {
            StrobeColor::Off => Color::OFF,
            StrobeColor::Color0 => state.color0(),
            StrobeColor::Color1 => state.color1(),
            StrobeColor::Strobe { pd, duty, alpha } => {
                let a = state.phi(pd).square(1.0, duty);
                state.color1().a(a * alpha)
            }
        }
    }
}
impl Strobes {
    pub fn new() -> Self {
        Self {
            color: StrobeColor::Off,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

pub struct Pads {
    pub pattern: PadPattern,
    pub brightness: f64,
}
#[derive(Clone, Copy, Debug)]
pub enum PadPattern {
    Color0,
    Color1,
}
#[async_trait]
impl Logic for Pads {
    async fn output(&self, state: &State, _: &mut Lights, pad: &Pad, _: &Ctrl) {
        let color0 = state.color0();
        let color1 = state.color1();
        match self.pattern {
            PadPattern::Color0 => pad.send(PadOutput::ClearColor(color0.into())).await,
            PadPattern::Color1 => pad.send(PadOutput::ClearColor(color1.into())).await,
        };
    }
}
impl Pads {
    pub fn new() -> Self {
        Self {
            pattern: PadPattern::Color0,
            brightness: 1.0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
