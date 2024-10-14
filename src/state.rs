use anyhow::Result;
use itertools::Itertools;
use rand::seq::SliceRandom;
use stagebridge::color::{Rgb, Rgbw};
use stagebridge::dmx::device::laser_scan_30w::{LaserColor, LaserPattern};
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
use crate::utils::{Holdable, Pd};

///////////////////////// STATE /////////////////////////

#[derive(Default)]
pub struct State {
    /// Time since the last `tick()` in seconds
    pub dt: f64,
    /// Total time elapsed since startup in seconds
    pub t: f64,

    /// Current approximately matched BPM
    pub bpm: f64,
    /// Timestamps when the beatmatch button was tapped
    pub bpm_taps: Vec<f64>,
    /// Current fractional beat number in a 16 beat measure at the current `bpm`. Ranges from `0..16` and wraps around
    pub phi: f64,

    /// Color palette
    pub palette: Palette,
    /// Lighting mode
    pub mode: Mode,
    ///
    pub hold: Hold,

    /// Global brightness modifier
    pub brightness: f64,
}

#[derive(Debug, Default)]
pub enum Palette {
    /// Gradually cycling rainbow
    #[default]
    Rainbow,
    /// Solid color
    Solid(Rgbw),
}

#[derive(Debug, Default)]
pub enum Mode {
    /// All off
    Off,
    /// All on, solid color
    #[default]
    On,
    /// TODO: ???
    Hover,
    /// Flashing to the beat
    AutoBeat {
        /// How often to flash
        pd: Pd,
        /// Brightness range for each flash, from 0..1
        r: Range,
    },
    /// Single flash to a manual beat.
    Beat {
        /// Time of press
        t: f64,
        /// Duration of beat.
        pd: Pd,
        /// Brightness range of flash.
        r: Range,
    },
    /// Strobe lights
    Strobe { pd: Pd, duty: f64 },
}

#[derive(Debug, Default)]
pub enum Hold {
    #[default]
    None,

    ///
    Off,
}

impl State {
    pub fn new() -> Self {
        Self {
            bpm: 120.0,
            brightness: 0.1,
            palette: Palette::Rainbow,
            ..Default::default()
        }
    }
}

///////////////////////// PAD /////////////////////////

pub fn on_pad(s: &mut State, l: &mut Lights, event: launchpad_x::Input) {
    use self::Mode;
    use launchpad_x::{types::*, *};
    log::debug!("pad: {event:?}");

    //
    if let Some((x, y, b)) = match event {
        Input::Press(i, _) => Some((Coord::from(Pos::from(i)).0, Coord::from(Pos::from(i)).1, true)),
        Input::Release(i) => Some((Coord::from(Pos::from(i)).0, Coord::from(Pos::from(i)).1, false)),
        _ => None,
    } {
        match (x, y) {
            // append beatmatch
            (4, 7) => s.bpm_taps.push(s.t),
            // reset phase
            (5, 7) => s.phi = 0.0,
            // apply beatmatch
            (6, 7) => match s.bpm_taps.len() {
                0 => {}
                1 => s.bpm_taps.clear(),
                n => {
                    let dt = s.bpm_taps.drain(..).tuple_windows().map(|(t0, t1)| t1 - t0).sum::<f64>() / (n as f64 - 1.0);
                    let bpm = 60.0 / dt;
                    log::info!("Calculated bpm={:.2} from {n} samples", s.bpm);
                }
            },

            (1, 0) => s.mode = Mode::Off,
            (1, 1) => s.mode = Mode::On,
            (1, 2) => s.mode = Mode::Hover,
            (1, 3) => s.mode = Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() },
            (1, 4) => s.mode = Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() },

            (0, 0) => s.mode = Mode::Beat { t: s.t, pd: Pd(4, 1), r: (1.0..0.0).into() },
            (0, 1) => s.mode = Mode::Beat { t: s.t, pd: Pd(2, 1), r: (1.0..0.0).into() },
            (0, 2) => s.mode = Mode::Beat { t: s.t, pd: Pd(1, 1), r: (1.0..0.0).into() },
            (0, 3) => s.mode = Mode::Beat { t: s.t, pd: Pd(1, 2), r: (1.0..0.0).into() },
            (0, 4) => s.mode = Mode::Beat { t: s.t, pd: Pd(1, 4), r: (1.0..0.0).into() },

            (6, 0) => s.mode = Mode::Off,
            (6, 1) => s.mode = Mode::On,
            (6, 2) => s.mode = Mode::Hover,
            (6, 3) => s.mode = Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.2).into() },
            (6, 4) => s.mode = Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.2).into() },

            (7, 0) => s.mode = Mode::Beat { t: s.t, pd: Pd(4, 1), r: (1.0..0.0).into() },
            (7, 1) => s.mode = Mode::Beat { t: s.t, pd: Pd(2, 1), r: (1.0..0.0).into() },
            (7, 2) => s.mode = Mode::Beat { t: s.t, pd: Pd(1, 1), r: (1.0..0.0).into() },
            (7, 3) => s.mode = Mode::Beat { t: s.t, pd: Pd(1, 2), r: (1.0..0.0).into() },
            (7, 4) => s.mode = Mode::Beat { t: s.t, pd: Pd(1, 4), r: (1.0..0.0).into() },

            (7, 5) => s.mode = Mode::AutoBeat { pd: Pd(1, 4), r: (1.0..0.0).into() },
            (7, 6) => s.mode = Mode::Strobe { pd: Pd(1, 8), duty: 1.0 },
            (0, 5) => s.mode = Mode::AutoBeat { pd: Pd(1, 4), r: (1.0..0.0).into() },
            (0, 6) => s.mode = Mode::Strobe { pd: Pd(1, 8), duty: 1.0 },

            // slow presets
            // (3, 0) => {
            //     s.env(|_| 0.1);
            //     l.beam_pos = BeamPos::Down;
            // },
            // (3, 1) => {},
            // (3, 2) => {
            //     s.env(|s| s.phi(1, 1).ramp(1.0).inv().lerp(0.2..0.3));
            //     l.beam_pos = BeamPos::WaveY { pd: Pd(8, 1) };
            // },

            // // fast presets
            // (4, 0) => {
            //     s.env(|_| 0.4);
            //     l.beam_pos = BeamPos::WaveY { pd: Pd(8, 1) };
            // },
            // (4, 1) => {},
            // (4, 2) => {
            //     s.env(|s| s.phi(1, 1).ramp(1.0).inv().lerp(0.2..0.5));
            //     l.beam_pos = BeamPos::Square { pd: Pd(8, 1) };
            // },

            // colorz
            (_, 0) => match x {
                3 => s.palette = Palette::Solid(Rgbw::WHITE),
                4 => s.palette = Palette::Rainbow,
                5 => s.palette = Palette::Solid(Rgbw::VIOLET),
                _ => {}
            },
            (_, 1) => match x {
                2 => s.palette = Palette::Solid(Rgbw::CYAN),
                3 => s.palette = Palette::Solid(Rgbw::BLUE),
                4 => s.palette = Palette::Solid(Rgbw::VIOLET),
                5 => s.palette = Palette::Solid(Rgbw::MAGENTA),
                _ => {}
            },
            (_, 2) => match x {
                2 => s.palette = Palette::Solid(Rgbw::RED),
                3 => s.palette = Palette::Solid(Rgbw::ORANGE),
                4 => s.palette = Palette::Solid(Rgbw::YELLOW),
                5 => s.palette = Palette::Solid(Rgbw::LIME),
                _ => {}
            },

            // (_, 3) => match x {
            //     2 => s.c1(Rgbw::BLACK),
            //     3 => s.c1(Rgbw::WHITE),
            //     4 => s.c1 = Op::f(|s| Rgbw::hsv(s.phi(16, 1), 1.0, 1.0)),
            //     5 => s.c0(Rgbw::WHITE),
            //     _ => {}
            // },
            // (_, 4) => match x {
            //     2 => s.c1(Rgbw::CYAN),
            //     3 => s.c1(Rgbw::BLUE),
            //     4 => s.c1(Rgbw::VIOLET),
            //     5 => s.c1(Rgbw::MAGENTA),
            //     _ => {}
            // },
            // (_, 5) => match x {
            //     2 => s.c1(Rgbw::RED),
            //     3 => s.c1(Rgbw::ORANGE),
            //     4 => s.c1(Rgbw::YELLOW),
            //     5 => s.c1(Rgbw::LIME),
            //     _ => {}
            // },
            // (_, 6) => match x {
            //     2 => s.cc(Rgbw::BLACK, Rgbw::BLACK),
            //     3 => s.cc(Rgbw::WHITE, Rgbw::WHITE),
            //     4 => s.ccc(|s| Rgbw::hsv(s.phi(16, 1), 1.0, 1.0)),
            //     5 => s.c0(Rgbw::VIOLET),
            //     _ => {}
            // },

            // hold pressure env
            // (6, 2) => s.beat0 = Beat::Fr(fr.in_exp()),
            // (7, 2) => s.beat0 = Beat::Fr(fr.in_exp()),

            // (0, 7) => s.mode1 = Mode::Press { fr: fr.in_exp() },
            // (7, 7) => s.mode0 = Mode::Press { fr: fr.in_exp() },

            // hold mod colors
            // (5, 1) => {
            //     s.c_h.hold(x, y, b, Op::f(|s| Rgbw::hsv(s.pd(Pd(4, 1)), 1.0, 1.0)));
            //     s.env_h.hold(x, y, b, Op::v(1.0));
            // },
            // (5, 2) => s.c_h.hold(x, y, b, Op::v(Rgbw::BLACK)),
            // (5, 3) => {
            //     s.c_h.hold(x, y, b, Op::v(Rgbw::WHITE));
            //     s.env_h.hold(x, y, b, Op::v(1.0));
            // },

            // hold strobe w/ pressure
            // (6, 3) => s.env_h.hold(x, y, b, Op::f(move |s| s.pd(Pd(1, 4)).square(1.0, fr.in_exp().lerp(1.0..0.5)))),
            // (7, 3) => s.env_h.hold(x, y, b, Op::f(move |s| s.pd(Pd(1, 8)).square(1.0, fr.in_exp().lerp(1.0..0.5)))),

            // hold white strobe
            // (6, 4) => {
            //     s.env_h0.hold(x, y, b, Op::f(move |s| s.pd(Pd(1, 4)).square(1.0, fr.in_exp().lerp(1.0..0.5))));
            //     s.env_h1.hold(x, y, b, Op::v(0.0));
            //     s.c_h.hold(x, y, b, Op::v(Rgbw::WHITE));
            // },
            // (7, 4) => {
            //     s.env_h.hold(x, y, b, Op::f(move |s| s.pd(Pd(1, 8)).square(1.0, fr.in_exp().lerp(1.0..0.5))));
            //     s.c_h.hold(x, y, b, Op::v(Rgbw::WHITE));
            // },

            // // hold chase
            // (6, 5) => {
            //     s.par_src_h.hold(x, y, b, Source::Chase { pd: Pd(1, 1), duty: 0.1 });
            //     s.env_h1.hold(x, y, b, Op::v(0.0));
            //     s.c_h.hold(x, y, b, Op::v(Rgbw::WHITE));
            //     s.strobe_src_h.hold(x, y, b, Source::Strobe { pd: Pd(1, 4), duty: fr.in_exp().lerp(1.0..0.5) });
            // }
            _ => {}
        }
    }
}

///////////////////////// CTRL /////////////////////////

pub fn on_ctrl(s: &mut State, l: &mut Lights, input: launch_control_xl::Input) {
    use launch_control_xl::{types::*, *};
    log::debug!("ctrl: {input:?}");

    match input {
        Input::Slider(0, fr) => s.brightness = fr,

        // laser tweaks
        Input::Focus(0, true) => l.laser.on = !l.laser.on,
        Input::Slider(1, fr) => l.laser.pattern = LaserPattern::Raw(fr.byte()),
        Input::Slider(2, fr) => l.laser.rotate = fr,
        Input::Slider(3, fr) => l.laser.x = fr,
        Input::Slider(4, fr) => l.laser.y = fr,
        Input::Slider(5, fr) => l.laser.size = fr,
        Input::Slider(6, fr) => l.laser.color = LaserColor::Raw(fr.byte()),

        // Input::Slider(3, fr) => l.laser.xflip = fr,
        // Input::Slider(4, fr) => l.laser.yflip = fr,
        _ => {}
    }
}

///////////////////////// TICK /////////////////////////

pub fn tick(dt: f64, s: &mut State, l: &mut Lights) {
    s.dt = dt;
    s.t += dt;
    s.phi = (s.phi + (dt * (s.bpm / 60.0))).fmod(16.0);
}

///////////////////////// LIGHTS /////////////////////////

pub fn render_lights(s: &mut State, l: &mut Lights) {
    l.reset();

    let color = match s.palette {
        Palette::Rainbow => Rgb::hsv(s.phi(16, 1), 1.0, 1.0).into(),
        Palette::Solid(col) => col,
    };
    log::debug!("color={color:?}");

    match s.mode {
        Mode::Off => {}
        Mode::On => {
            for par in &mut l.pars {
                par.color = color;
            }
        }
        _ => {}
    }

    l.send();
}

///////////////////////// PAD /////////////////////////

pub fn render_pad(s: &mut State, pad: &mut Midi<LaunchpadX>) {
    use self::Mode;
    use launchpad_x::{types::*, *};

    let mut batch: Vec<(Pos, Color)> = vec![];

    let mut rgb = |x, y, Rgb(r, g, b): Rgb| batch.push((Coord(x, y).into(), Color::Rgb(r, g, b)));

    for i in 1..=6 {
        rgb(i, 7, Rgb::VIOLET);
    }

    // mod colors
    rgb(2, 6, Rgb::BLACK);
    rgb(3, 6, Rgb::WHITE);
    rgb(4, 6, Rgb::hsv(s.phi(16, 1), 1.0, 1.0));
    rgb(5, 6, Rgb::WHITE);

    // color blocks
    for i in 2..=5 {
        rgb(i, 1, Rgb::WHITE);
        rgb(i, 2, Rgb::WHITE);
    }

    for i in 0..=1 {
        rgb(2, 3 * i + 2, Rgb::RED);
        rgb(3, 3 * i + 2, Rgb::ORANGE);
        rgb(4, 3 * i + 2, Rgb::YELLOW);
        rgb(5, 3 * i + 2, Rgb::LIME);
        rgb(2, 3 * i + 1, Rgb::CYAN);
        rgb(3, 3 * i + 1, Rgb::BLUE);
        rgb(4, 3 * i + 1, Rgb::VIOLET);
        rgb(5, 3 * i + 1, Rgb::MAGENTA);
        rgb(2, 3 * i, Rgb::BLACK);
        rgb(3, 3 * i, Rgb::WHITE);
        rgb(4, 3 * i, Rgb::hsv(s.phi(16, 1), 1.0, 1.0));
        rgb(5, 3 * i, Rgb::WHITE);
    }

    // // basic modes
    // for (i, c) in [(0, c11), (1, c00)] {
    //     rgb(1 + 5 * i, 0, Rgb::BLACK);
    //     rgb(1 + 5 * i, 1, c.into());
    //     rgb(1 + 5 * i, 2, c.a(Mode::Hover.env(s)).into());
    // }

    // rgb(1, 3, c11.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
    // rgb(1, 4, c11.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());
    // rgb(6, 3, c00.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.2).into() }.env(s)).into());
    // rgb(6, 4, c00.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.2).into() }.env(s)).into());

    // rgb(0, 5, c11.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
    // rgb(0, 6, c11.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());
    // rgb(7, 5, c00.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
    // rgb(7, 6, c00.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());

    // beats
    for i in 0..=4 {
        rgb(0, i, Rgb::WHITE);
        rgb(7, i, Rgb::WHITE);
    }

    // if let Mode::Press { fr } = s.mode0 {
    //     rgb(7, 7, c0.into());
    // }
    // if let Mode::Press { fr } = s.mode1 {
    //     rgb(0, 7, c1.into());
    // }

    // outer ring c1
    // pad.send(Output::Batch(
    //     (0..8)
    //         .flat_map(|i| [Coord(i, 8), Coord(8, i)])
    //         .map(|coord| (coord.into(), c1.into()))
    //         .collect(),
    // ));

    // alpha selectors
    for i in 0..=7 {
        rgb(i, 8, Rgb::WHITE);
        rgb(8, i, Rgb::VIOLET);
    }

    // beat, phi=0 indicator
    rgb(
        8,
        8,
        match s.pd(Pd(1, 1)).bsquare(1.0, 0.1) {
            true => match s.pd(Pd(4, 1)).bsquare(1.0, 0.2) {
                true => Rgb::VIOLET,
                false => Rgb::WHITE,
            },
            false => Rgb::BLACK,
        },
    );

    pad.send(Output::Batch(batch));
}

///////////////////////// CTRL /////////////////////////

pub fn render_ctrl(s: &mut State, pad: &mut Midi<LaunchControlXL>) {
    use self::Mode;
    use launch_control_xl::{types::*, *};

    for i in 0..8 {
        pad.send(Output::Control(
            i,
            Color::Red,
            match (s.phi - i as f64 * 0.125).tri(2.0).in_sin() {
                ..0.25 => Brightness::Off,
                0.25..0.5 => Brightness::Low,
                0.5..0.75 => Brightness::Medium,
                _ => Brightness::High,
            },
        ))
    }
}

///////////////////////// HELPERS /////////////////////////

impl State {
    fn phi(&self, n: usize, d: usize) -> f64 {
        self.pd(Pd(n, d))
    }
    fn pd(&self, pd: Pd) -> f64 {
        self.phi.fmod_div(pd.fr())
    }

    fn dt(&self, n: usize, d: usize) -> f64 {
        self.dt / ((self.bpm / 60.0) * Pd(n, d).fr())
    }

    // fn c(&mut self, c: Rgbw) {
    //     self.cc(c, c);
    // }
    // fn cc(&mut self, c0: Rgbw, c1: Rgbw) {
    //     self.c0 = Op::v(c0);
    //     self.c1 = Op::v(c1);
    // }
    // fn c0(&mut self, c0: Rgbw) {
    //     self.c0 = Op::v(c0);
    // }
    // fn c1(&mut self, c1: Rgbw) {
    //     self.c1 = Op::v(c1);
    // }
    // fn ccc(&mut self, f: impl Fn(&mut State) -> Rgbw + 'static) {
    //     self.c0 = Op::f(f);
    //     self.c1 = self.c0.clone();
    // }
}

// impl Mode {
//     fn env(self, s: &State) -> f64 {
//         match self {
//             Mode::Off => 0.0,
//             Mode::On => 1.0,
//             Mode::Hover => s.phi(8, 1).ssin(1.0).map(-1.0..1.0, 0.3..0.7),
//             Mode::AutoBeat { pd, r } => s.pd(pd).ramp(1.0).lerp(r).in_quad(),
//             Mode::Beat { t, pd, r } => {
//                 let dt = s.t - t;
//                 let len = (60.0 / s.bpm) * pd.fr();

//                 if dt >= len {
//                     r.hi
//                 } else {
//                     (dt / len).ramp(1.0).lerp(r).in_quad()
//                 }
//             }
//             Mode::Press { fr } => fr,
//             Mode::Strobe { pd, duty } => s.pd(pd).square(1.0, duty.in_exp().lerp(1.0..0.5)),
//         }
//     }
// }

///////////////////////// IO /////////////////////////

// fn io(
//     ctx: &egui::Context,
//     dt: f64,
//     s: &mut State,
//     l: &mut Lights,
//     pad: &mut Option<Midi<LaunchpadX>>,
//     ctrl: &mut Option<Midi<LaunchControlXL>>,
//     e131: &mut E131,
// ) {
//     if let Some(pad) = pad.as_mut() {
//         for event in pad.recv() {
//             on_pad(event, s, l, pad);
//         }
//     }
//     if let Some(ctrl) = ctrl.as_mut() {
//         for event in ctrl.recv() {
//             on_ctrl(event, s, l, ctrl);
//         }
//     }

//     // real time
//     s.dt0 = dt;
//     s.t0 += s.dt0;
//     s.phi0 = (s.phi0 + (s.dt0 * (s.bpm / 60.0))).fmod(16.0);
//     // modified time
//     s.dt = s.dt0 * s.t_mod;
//     s.t += s.dt;
//     s.phi = (s.phi + (s.dt * (s.bpm / 60.0))).fmod(16.0);

//     // tick alpha up/down
//     // s.alpha = if s.alpha_up { (s.alpha + s.dt(1, 1)) } else { s.alpha };
//     // s.alpha = if s.alpha_down { (s.alpha - s.dt(1, 1)).max(0.0) } else { s.alpha };

//     // apply hold fallbacks
//     // let env0 = s.env_h.or(&s.env_h0.or(&s.env0));
//     // let env1 = s.env_h.or(&s.env_h1.or(&s.env1));
//     // let c0 = s.c_h.or(&s.c_h0.or(&s.c0));
//     // let c1 = s.c_h.or(&s.c_h1.or(&s.c1));

//     // global alpha
//     // let a = s.alpha;

//     // envelope alpha
//     // let env0 = env0(s);
//     // let env1 = env1(s);
//     // let a0 = (a * s.beat0.or(s, env0)).min(1.0);
//     // let a1 = (a * s.beat1.or(s, env1)).min(1.0);

//     // alpha adjust colors
//     // let c0 = c0(s).a(a0);
//     // let c1 = c1(s).a(a1);

//     let env0 = (s.mode0.env(s) * s.alpha).max(s.min).min(s.max);
//     let env1 = (s.mode1.env(s) * s.alpha).max(s.min).min(s.max);

//     let c00 = (s.c0.clone())(s);
//     let c11 = (s.c1.clone())(s);

//     let c0 = c00.a(env0);
//     let c1 = c11.a(env1);

//     // pad stuff
//     if let Some(pad) = pad.as_mut() {
//         use self::Mode;
//         use launchpad_x::{types::*, *};

//         let mut batch: Vec<(Pos, Color)> = vec![];

//         let mut rgb = |x, y, Rgb(r, g, b): Rgb| batch.push((Coord(x, y).into(), Color::Rgb(r, g, b)));

//         for i in 1..=6 {
//             rgb(i, 7, Rgb::VIOLET);
//         }

//         // mod colors
//         rgb(2, 6, Rgb::BLACK);
//         rgb(3, 6, Rgb::WHITE);
//         rgb(4, 6, Rgb::hsv(s.phi(16, 1), 1.0, 1.0));
//         rgb(5, 6, Rgb::WHITE);

//         // color blocks
//         for i in 2..=5 {
//             rgb(i, 1, Rgb::WHITE);
//             rgb(i, 2, Rgb::WHITE);
//         }

//         for i in 0..=1 {
//             rgb(2, 3 * i + 2, Rgb::RED);
//             rgb(3, 3 * i + 2, Rgb::ORANGE);
//             rgb(4, 3 * i + 2, Rgb::YELLOW);
//             rgb(5, 3 * i + 2, Rgb::LIME);
//             rgb(2, 3 * i + 1, Rgb::CYAN);
//             rgb(3, 3 * i + 1, Rgb::BLUE);
//             rgb(4, 3 * i + 1, Rgb::VIOLET);
//             rgb(5, 3 * i + 1, Rgb::MAGENTA);
//             rgb(2, 3 * i, Rgb::BLACK);
//             rgb(3, 3 * i, Rgb::WHITE);
//             rgb(4, 3 * i, Rgb::hsv(s.phi(16, 1), 1.0, 1.0));
//             rgb(5, 3 * i, Rgb::WHITE);
//         }

//         // basic modes
//         for (i, c) in [(0, c11), (1, c00)] {
//             rgb(1 + 5 * i, 0, Rgb::BLACK);
//             rgb(1 + 5 * i, 1, c.into());
//             rgb(1 + 5 * i, 2, c.a(Mode::Hover.env(s)).into());
//         }

//         rgb(1, 3, c11.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
//         rgb(1, 4, c11.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());
//         rgb(6, 3, c00.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.2).into() }.env(s)).into());
//         rgb(6, 4, c00.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.2).into() }.env(s)).into());

//         rgb(0, 5, c11.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
//         rgb(0, 6, c11.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());
//         rgb(7, 5, c00.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
//         rgb(7, 6, c00.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());

//         // beats
//         for i in 0..=4 {
//             rgb(0, i, Rgb::WHITE);
//             rgb(7, i, Rgb::WHITE);
//         }

//         if let Mode::Press { fr } = s.mode0 {
//             rgb(7, 7, c0.into());
//         }
//         if let Mode::Press { fr } = s.mode1 {
//             rgb(0, 7, c1.into());
//         }

//         // outer ring c1
//         // pad.send(Output::Batch(
//         //     (0..8)
//         //         .flat_map(|i| [Coord(i, 8), Coord(8, i)])
//         //         .map(|coord| (coord.into(), c1.into()))
//         //         .collect(),
//         // ));

//         // alpha selectors
//         for i in 0..=7 {
//             rgb(i, 8, Rgb::WHITE);
//             rgb(8, i, Rgb::VIOLET);
//         }

//         // beat, phi=0 indicator
//         rgb(
//             8,
//             8,
//             match s.pd(Pd(1, 1)).bsquare(1.0, 0.1) {
//                 true => match s.pd(Pd(4, 1)).bsquare(1.0, 0.2) {
//                     true => Rgb::VIOLET,
//                     false => Rgb::WHITE,
//                 },
//                 false => Rgb::BLACK,
//             },
//         );

//         pad.send(Output::Batch(batch));
//     }

//     // lights
//     // l.par_src = s.par_src_h.or(&s.par_src);
//     // l.strobe_src = s.strobe_src_h.or(&s.strobe_src);
//     l.tick(s, c0, c1);

//     for beam in &mut l.beams {
//         beam.pitch = 1.0 - beam.pitch;
//     }

//     l.send(e131);
// }

///////////////////////// HELPERS /////////////////////////

// impl State {
//     fn phi(&self, n: usize, d: usize) -> f64 {
//         self.pd(Pd(n, d))
//     }
//     fn pd(&self, pd: Pd) -> f64 {
//         self.phi.fmod_div(pd.fr() * self.phi_mod)
//     }

//     fn dt(&self, n: usize, d: usize) -> f64 {
//         self.dt / ((self.bpm / 60.0) * Pd(n, d).fr())
//     }

//     fn c(&mut self, c: Rgbw) {
//         self.cc(c, c);
//     }
//     fn cc(&mut self, c0: Rgbw, c1: Rgbw) {
//         self.c0 = Op::v(c0);
//         self.c1 = Op::v(c1);
//     }
//     fn c0(&mut self, c0: Rgbw) {
//         self.c0 = Op::v(c0);
//     }
//     fn c1(&mut self, c1: Rgbw) {
//         self.c1 = Op::v(c1);
//     }
//     fn ccc(&mut self, f: impl Fn(&mut State) -> Rgbw + 'static) {
//         self.c0 = Op::f(f);
//         self.c1 = self.c0.clone();
//     }
// }

// impl Mode {
//     fn env(self, s: &State) -> f64 {
//         match self {
//             Mode::Off => 0.0,
//             Mode::On => 1.0,
//             Mode::Hover => s.phi(8, 1).ssin(1.0).map(-1.0..1.0, 0.3..0.7),
//             Mode::AutoBeat { pd, r } => s.pd(pd).ramp(1.0).lerp(r).in_quad(),
//             Mode::Beat { t, pd, r } => {
//                 let dt = s.t - t;
//                 let len = (60.0 / s.bpm) * pd.fr();

//                 if dt >= len {
//                     r.hi
//                 } else {
//                     (dt / len).ramp(1.0).lerp(r).in_quad()
//                 }
//             }
//             Mode::Press { fr } => fr,
//             Mode::Strobe { pd, duty } => s.pd(pd).square(1.0, duty.in_exp().lerp(1.0..0.5)),
//         }
//     }
// }
