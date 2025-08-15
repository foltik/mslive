use stagebridge::{
    color::{Rgb, Rgbw},
    midi::{
        device::{
            launch_control_xl,
            launchpad_x::{self, LaunchpadX},
        },
        Midi,
    },
    num::Interp,
};

use crate::lights::Lights;
use crate::pages::Page;
use crate::utils::Swatch;

#[derive(Clone, Copy, Debug)]
pub enum Preset {
    Off,
    House,
    Sine,
    Dimmers,
}

const PALETTE: &[Swatch<Preset>] = &[
    Swatch { xy: (0, 7), op: Preset::Off },
    Swatch { xy: (1, 7), op: Preset::House },
    Swatch { xy: (2, 7), op: Preset::Dimmers },
    Swatch { xy: (0, 5), op: Preset::Sine },
];

pub struct Presets {
    pub preset: Preset,
    pub time: f64,
    speed: f64,
}

impl Preset {
    fn color(&self, s: &Presets) -> Rgb {
        match self {
            Preset::Off => Rgb::WHITE * 0.1,
            Preset::House => Rgb::WHITE,
            Preset::Dimmers => Rgb::WHITE,
            Preset::Sine => Rgb::WHITE * s.time.fsin(3.0),
        }
    }

    fn render(&self, s: &Presets, l: &mut Lights) {
        match self {
            Preset::Off => {}
            Preset::House => {
                for rgbw in &mut l.rgbw {
                    *rgbw = Rgbw::HOUSE;
                }
                for dimmer in &mut l.dimmer {
                    *dimmer = 1.0;
                }
            }
            Preset::Dimmers => {
                // for dimmer in &mut l.dimmer {
                //     *dimmer = 1.0;
                // }
                l.bar.angle = s.time.fsin(3.0);
                for bead in &mut l.bar.beads {
                    *bead = Rgbw::WHITE;
                }
            }
            Preset::Sine => {
                let fr = s.time.fsin(3.0);
                for rgbw in &mut l.rgbw {
                    *rgbw = Rgbw::HOUSE * fr;
                }
            }
        }
    }
}

impl Default for Presets {
    fn default() -> Self {
        Self { preset: Preset::Off, time: 0.0, speed: 1.0 }
    }
}


impl Page for Presets {
    fn tick(&mut self, dt: f64) {
        self.time += dt;
    }

    fn input_pad(&mut self, _pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
        let Some((x, y)) = event.xy() else { return };

        if let Some(swatch) = PALETTE.iter().find(|s| s.xy == (x, y)) {
            self.preset = swatch.op;
        }
    }

    fn input_ctrl(&mut self, _event: launch_control_xl::Input) {}

    fn output_lights(&self, lights: &mut Lights) {
        lights.reset();
        self.preset.render(self, lights);
    }

    fn output_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};

        let mut batch: Vec<(Pos, Color)> = Vec::with_capacity(PALETTE.len());
        for Swatch { xy: (x, y), op } in PALETTE.iter().copied() {
            let Rgb(r, g, b) = op.color(self);
            batch.push((Coord(x, y).into(), Color::Rgb(r, g, b)));
        }
        pad.send(Output::Batch(batch));
    }

    fn output_ctrl(&self, _ctrl: &mut Midi<launch_control_xl::LaunchControlXL>) {}
}
