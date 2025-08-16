#![allow(unused)]

use std::collections::HashSet;

use egui::Key;
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

///////////////////////// PRESETS /////////////////////////

palette! {
    Key::Num1; (0, 7) => Off,
    Key::Num2; (1, 7) => House,
    Key::Num3; (2, 7) => Dimmers,
    Key::Num4; (3, 7) => WhiteBar,
    Key::Num5; (0, 5) => Sine,
    Key::Num6; (7, 7) => Random,
}

struct Random;
impl Preset for Random {
    fn color(&self, s: &Presets) -> Rgb {
        // color of the button on the launchpad
        Rgb::WHITE
    }

    fn lights(&self, s: &Presets, l: &mut Lights) {
        // s.t is a float representing the number of seconds elapsed since program start
        // the fract() returns the decimal part, so 123.456 would return 0.456
        let t = s.time.fract();

        // this color alternates between WHITE and BLACK every 0.5s
        let color = if t > 0.5 { Rgbw::WHITE } else { Rgbw::BLACK };

        // l.rgbw: an array of colors that gets mapped to each RGB bar.
        // for example: l.rgbw[0] is an Rgbw type: Rgbw(0.5, 1.0, 0.25, 0.3),
        // where each number is the red, green, blue, white channel respectively
        for rgbw_color in &mut l.rgbw {
            *rgbw_color = color;
        }

        // l.dimmer: an array of brightnesses from 0.0 to 1.0 for each dimmer pack
        // for example: l.dimmer[0] is a float
        for dimmer_brightness in &mut l.dimmer {
            *dimmer_brightness = 1.0;
        }

        // to see all the available properties on `l`, go to `lights.rs`
    }
}

struct Off;
impl Preset for Off {
    fn color(&self, s: &Presets) -> Rgb {
        Rgb::BLACK
    }
    fn lights(&self, s: &Presets, l: &mut Lights) {}
}

struct House;
impl Preset for House {
    fn color(&self, s: &Presets) -> Rgb {
        Rgb::HOUSE
    }

    fn lights(&self, s: &Presets, l: &mut Lights) {
        for rgbw in &mut l.rgbw {
            *rgbw = Rgbw::HOUSE;
        }
        for dimmer in &mut l.dimmer {
            *dimmer = 1.0;
        }
    }
}

struct Dimmers;
impl Preset for Dimmers {
    fn color(&self, s: &Presets) -> Rgb {
        Rgb::WHITE
    }

    fn lights(&self, s: &Presets, l: &mut Lights) {
        for dimmer in &mut l.dimmer {
            *dimmer = 1.0;
        }
    }
}

struct WhiteBar;
impl Preset for WhiteBar {
    fn color(&self, s: &Presets) -> Rgb {
        Rgb::WHITE
    }

    fn lights(&self, s: &Presets, l: &mut Lights) {
        l.bar.angle = s.time.fsin(3.0);
        for bead in &mut l.bar.beads {
            *bead = Rgbw::WHITE;
        }
    }
}

struct Sine;
impl Preset for Sine {
    fn color(&self, s: &Presets) -> Rgb {
        Rgb::WHITE * s.time.fsin(3.0)
    }

    fn lights(&self, s: &Presets, l: &mut Lights) {
        let fr = s.time.fsin(3.0);
        for rgbw in &mut l.rgbw {
            *rgbw = Rgbw::HOUSE * fr;
        }
    }
}

///////////////////////// PAGE /////////////////////////

trait Preset {
    fn color(&self, s: &Presets) -> Rgb;
    fn lights(&self, s: &Presets, l: &mut Lights);
}

pub struct Presets {
    presets: Vec<(Key, Swatch<Box<dyn Preset>>)>,
    preset: usize,
    time: f64,
}

impl Default for Presets {
    fn default() -> Self {
        Self { presets: presets(), preset: 0, time: 0.0 }
    }
}

impl Page for Presets {
    fn tick(&mut self, dt: f64) {
        self.time += dt;
    }

    fn input_ctrl(&mut self, _event: launch_control_xl::Input) {}
    fn input_pad(&mut self, _pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
        let Some((x, y)) = event.xy() else { return };

        if let Some((i, _swatch)) = self.presets.iter().enumerate().find(|(_i, (_k, s))| s.xy == (x, y)) {
            self.preset = i;
        }
    }
    fn input_keys(&mut self, keys: &HashSet<Key>) {
        for (i, (key, _)) in self.presets.iter().enumerate() {
            if keys.contains(key) {
                self.preset = i;
            }
        }
        // if keys.contains(&Key::Num1) {
        // }
    }

    fn output_lights(&self, lights: &mut Lights) {
        self.presets[self.preset].1.op.lights(self, lights);
    }
    fn output_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};

        let mut batch: Vec<(Pos, Color)> = Vec::with_capacity(self.presets.len());
        for (_key, Swatch { xy: (x, y), op }) in self.presets.iter() {
            let Rgb(r, g, b) = op.color(self);
            batch.push((Coord(*x, *y).into(), Color::Rgb(r, g, b)));
        }
        pad.send(Output::Batch(batch));
    }
    fn output_ctrl(&self, _ctrl: &mut Midi<launch_control_xl::LaunchControlXL>) {}
}

macro_rules! palette {
    ($($key:expr ; ($x:expr, $y:expr) => $preset:expr),* $(,)?) => {
        fn presets() -> Vec<(Key, Swatch<Box<dyn Preset>>)> {
            vec![
                $( ($key, Swatch { xy: ($x, $y), op: Box::new($preset) }) ),*
            ]
        }
    }
}
use palette;
