use async_trait::async_trait;

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
use crate::lights::Lights;
use crate::{State, Pd};

#[async_trait]
pub trait Logic: Sync {
    fn pad(&mut self, _state: &mut State, _input: PadInput) {}
    fn ctrl(&mut self, _state: &mut State, _input: CtrlInput) {}
    async fn output(&self, state: &State, lights: &mut Lights, pad: &Pad, ctrl: &Ctrl);
}

#[derive(Clone, Copy, Debug)]
pub enum ClockSource {
    Osc,
    Static { bpm: f64 },
}

pub struct Time {
    pub source: ClockSource
}
#[async_trait]
impl Logic for Time {
    fn pad(&mut self, _: &mut State, input: PadInput) {
        if let PadInput::Capture(true) = input {
            self.source = match self.source {
                ClockSource::Osc => ClockSource::Static { bpm: 120.0 },
                ClockSource::Static { .. } => ClockSource::Osc,
            }
        }
    }

    fn ctrl(&mut self, state: &mut State, input: CtrlInput) {
        match input {
            CtrlInput::Mute(true) => state.phi_mul = 0.5,
            CtrlInput::Solo(true) => state.phi_mul = 1.0,
            CtrlInput::Record(true) => state.phi_mul = 2.0,
            _ => {},
        }
    }

    async fn output(&self, state: &State, _: &mut Lights, pad: &Pad, _: &Ctrl) {
        pad.send(PadOutput::Light(
            Coord(8, 8).into(),
            match state.phi(Pd(1, 1)).bsquare(1.0, 0.05) {
                true => PaletteColor::White,
                false => PaletteColor::Off,
            },
        ))
        .await;
        pad.send(PadOutput::Light(
            Coord(7, 8).into(),
            match &self.source {
                ClockSource::Osc => PaletteColor::White,
                ClockSource::Static { .. } => PaletteColor::Off,
            },
        ))
        .await;
    }
}
impl Time {
    pub fn new() -> Self {
        Self {
            source: ClockSource::Osc,
            // source: ClockSource::Static { bpm: 120.0 }
        }
    }
}
