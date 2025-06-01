use anyhow::Result;
use itertools::Itertools;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;
use stagebridge::color::{Rgb, Rgbw};
use stagebridge::dmx::device::beam_rgbw_60w::Beam;
use stagebridge::dmx::device::beam_rgbw_90w::BigBeam;
use stagebridge::dmx::device::laser_scan_30w::{Laser, LaserColor, LaserPattern};
use stagebridge::dmx::device::spider_rgbw_8x10w::Spider;
use std::time::Instant;
use std::{thread, time::Duration};

use stagebridge::e131::E131;
use stagebridge::midi::device::{
    launch_control_xl::{self, LaunchControlXL},
    launchpad_x::{self, LaunchpadX},
};
use stagebridge::midi::Midi;
use stagebridge::prelude::*;

use crate::lights::Lights;
use crate::utils::{Hold, Pd};

///////////////////////// TODO /////////////////////////

///////////////////////// IDEAS /////////////////////////

///////////////////////// STATE /////////////////////////

#[rustfmt::skip]
#[derive(Default)]
pub struct State {
    /// Time since the last `tick()` in seconds
    pub dt: f64,
    /// Total time elapsed since startup in seconds
    pub t0: f64,
    /// Total time elapsed since startup in seconds, modified by speed
    pub t: f64,

    /// Current approximately matched BPM
    pub bpm: f64,
    /// Timestamps when the beatmatch button was tapped
    pub bpm_taps: Vec<f64>,
    /// Current fractional beat number in a 64 beat measure at the current `bpm`. Ranges from `0..64` and wraps around
    pub phi: f64,
    /// Bpm multiplier, e.g. 0.5 for half-time, 2.0 for double-time.
    pub phi_mul: f64,

    /* Global Settings */
    /// Control mode
    pub mode: Mode,
    /// House lights
    pub house: f64,
    /// Global alpha
    pub brightness: f64,
    /// Global speed modifiers
    pub speed_coarse: f64,
    pub speed_fine: f64,
    /// Global decay modifier
    pub decay: f64,
    /// Generic modifiers depending on patterns
    pub sliders: [f64; 8],
    pub knobs: [f64; 8],

    /* Patterns */
    pub color: Color,
    pub rgbw_pat: RgbwPattern,
    pub cage_pat: CagePattern,
    pub dimmer_pat: DimmerPattern,

    /* Jam Rgbw */
    /* Jam Dimmer */
}

impl State {
    pub fn new() -> Self {
        Self {
            brightness: 1.0,
            bpm: 120.0,
            phi_mul: 1.0,
            speed_coarse: 1.0,
            decay: 0.5,
            ..Default::default()
        }
    }

    fn phi(&self, n: usize, d: usize) -> f64 {
        self.pd(Pd(n, d))
    }
    fn pd(&self, pd: Pd) -> f64 {
        self.phi.fmod_div(pd.fr())
    }

    fn dt(&self, n: usize, d: usize) -> f64 {
        self.dt / ((self.bpm / 60.0) * Pd(n, d).fr())
    }
}

///////////////////////// Mode /////////////////////////

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Mode {
    #[default]
    Patterns,
    JamRgbw,
    JamDimmers,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum RgbwPattern {
    #[default]
    Off,
    Chase,
    ChaseSine,
    ChaseRandom,
    Strobe,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum CagePattern {
    #[default]
    ChaseSine,
    ChaseRandom,
    Strobe,
    SyncBounce,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum DimmerPattern {
    #[default]
    Off,
    Manual,
    Chase,
    Random,
    Circle,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Color {
    #[default]
    House,

    White,
    Red,
    Green,
    Blue,
    Purple,

    RedWhite,
    GreenWhite,
    BlueWhite,

    Rainbow,
    ManualHue,
    ManualRgbw,
}

///////////////////////// LIGHTS /////////////////////////

pub fn render_lights(s: &mut State, l: &mut Lights) {
    for i in 0..9 {
        l.rgbw[i] = Rgbw::ORANGE * s.t.fsin(8.0) * 0.5;
    }

    l.send();
    return;

    if s.house > 0.0 {
        for i in 0..9 {
            l.rgbw[i] = Rgbw(0.39, 0.19, 0.0, 0.0) * s.house;
        }
        return;
    }

    l.reset();

    /* Rgbw */
    match s.mode {
        Mode::JamRgbw => {}
        Mode::Patterns => {}
        _ => {}
    }

    /* Dimmers */
    match s.mode {
        Mode::JamDimmers => {}
        Mode::Patterns => {}
        _ => {}
    }

    l.send();
}

///////////////////////// PAD /////////////////////////

pub fn render_pad(s: &mut State, l: &Lights, pad: &mut Midi<LaunchpadX>) {
    use self::Mode;
    use launchpad_x::{types::*, *};

    let mut batch: Vec<(Pos, Color)> = vec![];

    // Helper to set an x/y coord to a certain color
    let rgb = |Rgb(r, g, b): Rgb| Color::Rgb(r, g, b);
    let mut set = |x, y, color: Rgb| batch.push((Coord(x, y).into(), rgb(color)));

    // match s.mode {
    //     _ => {
    //         for (n, &fr) in l.dimmers.iter().enumerate() {
    //             let x = n % 3;
    //             let y = n / 3;

    //             for i in (3 * x)..(3 * (x + 1)) {
    //                 for j in (3 * y)..(3 * (y + 1)) {
    //                     set(i as i8, j as i8, Rgb::WHITE * fr);
    //                 }
    //             }
    //         }
    //     }
    // }

    pad.send(Output::Batch(batch));
}

///////////////////////// CTRL /////////////////////////

#[rustfmt::skip]
pub fn render_ctrl(s: &mut State, ctrl: &mut Midi<LaunchControlXL>) {
    use launch_control_xl::{types::*, *};
    use crate::logic::Mode as Mode;
    let mut set = |i, b| ctrl.send(Output::Control(i, Color::Red, if b { Brightness::High } else { Brightness::Off }));

    // match s.mode {
    //     Mode::Off    => set(0, true),
    //     Mode::On     => set(1, true),
    //     Mode::Manual => set(2, true),
    //     Mode::Chase  => set(3, true),
    //     Mode::Random => set(4, true),
    //     Mode::Circle => set(5, true),
    //     Mode::Spiral => set(6, true),
    // }
}

///////////////////////// TICK /////////////////////////

pub fn tick(dt: f64, s: &mut State, l: &mut Lights) {
    s.dt = dt;
    s.t0 += dt;
    s.t += dt * s.speed_coarse;
    s.phi = (s.phi + (dt * (s.bpm / 60.0) * s.phi_mul)).fmod(64.0);

    // match s.mode {
    //     Mode::Patterns => {}
    //     Mode::JamRgbw => {}
    //     Mode::JamDimmers => {}
    // }

    // let target = (s.t).floor() as usize % 9;
    // log::info!("t={} target={target}", s.t);
    // for i in 0..9 {
    //     if i == target {
    //         // l.rgbw[i] = Rgb::hsv(s.phi(16, 1), 1.0, 1.0).into();
    //         l.rgbw[i] = Rgbw::BLUE * s.brightness;
    //     } else {
    //         l.rgbw[i] = Rgbw::BLACK;
    //     }
    // }

    // l.rgbw[0] = Rgbw(0.0, 0.0, 0.0, 1.0);
    // l.rgbw[0] = Rgb::hsv(s.phi(16, 1), 1.0, 1.0).into();
    // let target = (s.t).floor() as usize % 4;
    // for i in 0..9 {
    //     //     // log::info!("target={target}");
    //     //     // l.rgbw[i] = match target {
    //     //     //     0 => Rgbw::RED,
    //     //     //     1 => Rgbw::LIME,
    //     //     //     2 => Rgbw::BLUE,
    //     //     //     3 => Rgbw::WHITE,
    //     //     //     _ => unreachable!(),
    //     //     // };
    //     l.rgbw[i] = Rgbw::WHITE;
    // }

    // // Apply manual beats
    // for i in 0..9 {
    //     s.manual[i] -= s.dt * s.decay;
    //     l.dimmers[i] = l.dimmers[i].max(s.manual[i]);
    // }

    // log::info!("{:?}", l.dimmers);

    // match s.mode {
    //     Mode::On => {
    //         for i in 0..9 {
    //             l.dimmers[i] = s.brightness;
    //         }
    //         for i in 0..110 {
    //             l.test[i] = s.brightness;
    //         }
    //     }
    //     Mode::Off => {
    //         for i in 0..9 {
    //             l.dimmers[i] = 0.0;
    //         }
    //     }
    //     Mode::Manual => {
    //         for i in 0..9 {
    //             l.dimmers[i] = s.manual[i];
    //         }
    //         //
    //     }
    //     Mode::Sequential => {
    //         let target = (s.t).floor() as usize % 110;
    //         println!("{target}");
    //         for i in 1..109 {
    //             l.test[i] = if i == target { s.brightness } else { 0.0 };
    //         }
    //         // for i in 0..9 {
    //         //     l.dimmers[i] = if i == target { s.brightness } else { 0.0 };
    //         // }
    //     }
    //     Mode::Random => {
    //         let step = (s.t).floor() as usize;
    //         if step > s.rand_step {
    //             let prev_idx = s.rand_idx;
    //             while s.rand_idx == prev_idx {
    //                 s.rand_idx = rand::thread_rng().gen_range(0..9) as usize;
    //             }

    //             s.rand_step = step;

    //             for i in 0..9 {
    //                 l.dimmers[i] = if i == s.rand_idx { s.brightness } else { 0.0 };
    //             }
    //         }
    //     }
    //     Mode::Circle => {
    //         let target = (s.t).floor() as usize % 8;
    //         for i in 0..8 {
    //             let n = match i {
    //                 0 => 0,
    //                 1 => 1,
    //                 2 => 2,
    //                 3 => 5,
    //                 4 => 8,
    //                 5 => 7,
    //                 6 => 6,
    //                 7 => 3,
    //                 _ => unreachable!(),
    //             };

    //             l.dimmers[n] = if i == target { s.brightness } else { 0.0 };
    //         }
    //         l.dimmers[4] = 0.0;
    //     }
    //     Mode::Spiral => {}
    // }

    // let random_colors = || match ThreadRng::default().gen_range(1..=8) {
    //     1 => Colors::Solid(Rgbw::RED),
    //     2 => Colors::Split(Rgbw::WHITE, Rgbw::RED),
    //     3 => Colors::Solid(Rgbw::RED),
    //     4 => Colors::Split(Rgbw::WHITE, Rgbw::RED),
    //     5 => Colors::Solid(Rgbw::RED),
    //     6 => Colors::Split(Rgbw::WHITE, Rgbw::RED),
    //     7 | 8 | _ => Colors::Rainbow,
    // };
    // fn randomize<T: Copy + PartialEq>(value: &mut T, func: impl Fn() -> T) {
    //     loop {
    //         let v = func();
    //         if v != *value {
    //             *value = v;
    //             break;
    //         }
    //     }
    // }
}

///////////////////////// PAD INPUT /////////////////////////

pub fn on_pad(s: &mut State, l: &mut Lights, pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
    use launchpad_x::{types::*, *};
    log::debug!("pad: {event:?}");

    if let Some((x, y)) = match event {
        Input::Press(i, _) => Some((Coord::from(i).0, Coord::from(i).1)),
        _ => None,
    } {
        // let i = x / 3;
        // let j = y / 3;
        // let n = (j * 3) + i;
        // log::info!("Pressed i={i} j={j} n={n}");
        // s.manual[n as usize] = 1.0;
        // l.dimmers[n as usize] = 1.0;

        match (x, y) {
            // // Beatmatch
            // (0, 7) => s.bpm_taps.push(s.t),
            // // Beatmatch apply
            // (7, 7) => match s.bpm_taps.len() {
            //     // If no beats, just reset phase
            //     0 => s.phi = 0.0,
            //     1 => s.bpm_taps.clear(),
            //     n => {
            //         // Calculate time difference between each consecutive tap
            //         let dts = s.bpm_taps.drain(..).tuple_windows().map(|(t0, t1)| t1 - t0);
            //         // Average out the difference
            //         let dt = dts.sum::<f64>() / (n as f64 - 1.0);
            //         // Calculate BPM
            //         let bpm = 60.0 / dt;

            //         s.phi = 0.0;
            //         s.bpm = bpm;
            //         log::info!("Calculated bpm={bpm:.2} from {n} samples");
            //     }
            // },
            _ => {}
        }
    }
}

///////////////////////// CTRL INPUT /////////////////////////

#[rustfmt::skip]
pub fn on_ctrl(s: &mut State, l: &mut Lights, ctrl: &mut Midi<LaunchControlXL>, input: launch_control_xl::Input) {
    use launch_control_xl::{types::*, *};
    use crate::logic::Mode;
    log::debug!("ctrl: {input:?}");

    match input {
        Input::SendA(0, fr) => s.brightness = (fr * 0.5) + 0.5,
        Input::SendA(1, fr) => s.speed_coarse = (fr * 0.5) + 0.5,
        Input::SendA(2, fr) => s.speed_fine = (fr * 0.5) + 0.5,
        Input::SendA(7, fr) => s.house = (fr * 0.5) + 0.5,

        // Input::Slider(0, fr) => s.brightness = fr,
        // Input::Slider(1, fr) => s.speed = fr * 60.0,
        // Input::Slider(2, fr) => s.decay = fr * 3.0,
        // Input::Slider(3, fr) => l.test[0] = fr,
        //
        // Input::Slider(0, fr) => l.test[0] = fr,
        // Input::Slider(1, fr) => l.test[1] = fr,
        // Input::Slider(2, fr) => l.test[2] = fr,
        // Input::Slider(3, fr) => l.test[3] = fr,
        // Input::Slider(4, fr) => l.test[4] = fr,
        // Input::Slider(5, fr) => l.test[5] = fr,
        // Input::Slider(6, fr) => l.test[6] = fr,
        // Input::Slider(7, fr) => l.test[7] = fr,

        // laser tweaks
        // Input::Focus(0, true) => l.laser.on = !l.laser.on,
        // Input::Slider(1, fr) => {
        //     l.laser.pattern = LaserPattern::Raw(fr.byte());
        //     println!("{:?}", l.laser.pattern);
        // }
        // Input::Slider(2, fr) => l.laser.rotate = fr,
        // Input::Slider(3, fr) => l.laser.x = fr,
        // Input::Slider(4, fr) => l.laser.y = fr,
        // Input::Slider(5, fr) => l.laser.size = fr,
        // Input::Slider(6, fr) => l.laser.color = LaserColor::Raw(fr.byte()),

        // Input::Slider(3, fr) => l.laser.xflip = fr,
        // Input::Slider(4, fr) => l.laser.yflip = fr,
        _ => {}
    }
}
