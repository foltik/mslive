#![allow(unused)]
#![allow(clippy::single_match)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::needless_pass_by_ref_mut)]

#![feature(stmt_expr_attributes)]

use color_eyre::Result;
use itertools::Itertools;
use rand::seq::SliceRandom;
use stagebridge::color::{Rgbw, Rgb};
use stagebridge::dmx::device::laser_scan_30w::{LaserPattern, LaserColor};
use std::time::Instant;
use std::{thread, time::Duration};

use stagebridge::e131::{E131, E131_PORT};
use stagebridge::midi::device::{
    launch_control_xl::{self, LaunchControlXL},
    launchpad_x::{self, LaunchpadX},
};
use stagebridge::midi::Midi;
use stagebridge::num::{Float, Ease, Range};

mod lights; use lights::*;
mod types; use types::*;

///////////////////////// STATE /////////////////////////

#[derive(Clone, Default)]
pub struct State {
    // bpm
    bpm: f64,
    beatmatch: Vec<f64>,
    // time
    dt0: f64,
    t0: f64,
    phi0: f64,
    dt: f64,
    t: f64,
    phi: f64,
    // time modifiers
    t_mod: f64,
    phi_mod: f64,

    // color
    c0: Op<Rgbw>,
    c1: Op<Rgbw>,

    mode0: Mode,
    mode1: Mode,

    // light source / pos
    par_src: Source,
    // par_src_h: Hold<Source>,
    strobe_src: Source,
    // strobe_src_h: Hold<Source>,

    min: f64,
    max: f64,

    off: bool,
    alpha: f64,
    alpha_up: bool,
    alpha_down: bool,
}

#[derive(Clone, Copy, Default)]
enum Mode {
    Off,
    #[default]
    On,
    Hover,
    AutoBeat { pd: Pd, r: Range },
    Beat { t: f64, pd: Pd, r: Range },
    Press { fr: f64 },
    Strobe { pd: Pd, duty: f64 },

    // Preset,
}

enum Hold {
    Off,
    Strobe,

}

///////////////////////// PAD /////////////////////////

fn on_pad(
    event: launchpad_x::Input,
    s: &mut State,
    l: &mut Lights,
    pad: &mut Midi<LaunchpadX>,
) {
    use launchpad_x::{types::*, *};
    use self::Mode;

    match event {
        // alpha up/down hold
        // Input::Up(b) => s.alpha_up = b,
        // Input::Down(b) => s.alpha_down = b,
        _ => {},
    }

    println!("{event:?}");

    if let Some((x, y, b, fr, poly)) = match event {
        Input::Press(i, fr) => Some((Coord::from(Pos::from(i)).0, Coord::from(Pos::from(i)).1, true, fr, false)),
        Input::Release(i) => Some((Coord::from(Pos::from(i)).0, Coord::from(Pos::from(i)).1, false, 0.0, false)),
        Input::PolyPressure(i, fr) => Some((Coord::from(Pos::from(i)).0, Coord::from(Pos::from(i)).1, true, fr, true)),
        _ => None,
    } {
        if b && !poly {
            println!("Pad({x}, {y})");

            // taps
            match (x, y) {
                // append beatmatch
                (4, 7) => s.beatmatch.push(s.t0),
                // reset phase
                (5, 7) => s.phi = 0.0,
                // apply beatmatch
                (6, 7) => match s.beatmatch.len() {
                    0 => {},
                    1 => s.beatmatch.clear(),
                    n => {
                        let dt = s.beatmatch.drain(..)
                            .tuple_windows()
                            .map(|(t0, t1)| t1 - t0)
                            .sum::<f64>() / (n as f64 - 1.0);

                        s.bpm = 60.0 / dt;
                        println!("bpm={:.2} n={n}", s.bpm);
                    }
                }

                (1, 0) => s.mode1 = Mode::Off,
                (1, 1) => s.mode1 = Mode::On,
                (1, 2) => s.mode1 = Mode::Hover,
                (1, 3) => s.mode1 = Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() },
                (1, 4) => s.mode1 = Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() },

                (0, 0) => s.mode1 = Mode::Beat { t: s.t, pd: Pd(4, 1), r: (1.0..0.0).into() },
                (0, 1) => s.mode1 = Mode::Beat { t: s.t, pd: Pd(2, 1), r: (1.0..0.0).into() },
                (0, 2) => s.mode1 = Mode::Beat { t: s.t, pd: Pd(1, 1), r: (1.0..0.0).into() },
                (0, 3) => s.mode1 = Mode::Beat { t: s.t, pd: Pd(1, 2), r: (1.0..0.0).into() },
                (0, 4) => s.mode1 = Mode::Beat { t: s.t, pd: Pd(1, 4), r: (1.0..0.0).into() },

                (6, 0) => s.mode0 = Mode::Off,
                (6, 1) => s.mode0 = Mode::On,
                (6, 2) => s.mode0 = Mode::Hover,
                (6, 3) => s.mode0 = Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.2).into() },
                (6, 4) => s.mode0 = Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.2).into() },

                (7, 0) => s.mode0 = Mode::Beat { t: s.t, pd: Pd(4, 1), r: (1.0..0.0).into() },
                (7, 1) => s.mode0 = Mode::Beat { t: s.t, pd: Pd(2, 1), r: (1.0..0.0).into() },
                (7, 2) => s.mode0 = Mode::Beat { t: s.t, pd: Pd(1, 1), r: (1.0..0.0).into() },
                (7, 3) => s.mode0 = Mode::Beat { t: s.t, pd: Pd(1, 2), r: (1.0..0.0).into() },
                (7, 4) => s.mode0 = Mode::Beat { t: s.t, pd: Pd(1, 4), r: (1.0..0.0).into() },

            (7, 5) => s.mode0 = Mode::AutoBeat { pd: Pd(1, 4), r: (1.0..0.0).into() },
            (7, 6) => s.mode0 = Mode::Strobe { pd: Pd(1, 8), duty: 1.0 },//duty: fr.in_exp() },
            (0, 5) => s.mode1 = Mode::AutoBeat { pd: Pd(1, 4), r: (1.0..0.0).into() },
            (0, 6) => s.mode1 = Mode::Strobe { pd: Pd(1, 8), duty: 1.0 },//duty: fr.in_exp() },


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
                    2 => s.c0(Rgbw::BLACK),
                    3 => s.c0(Rgbw::WHITE),
                    4 => s.c0 = Op::f(|s| Rgbw::hsv(s.phi(16, 1), 1.0, 1.0)),
                    5 => s.c0(Rgbw::VIOLET),
                    _ => {},
                },
                (_, 1) => match x {
                    2 => s.c0(Rgbw::CYAN),
                    3 => s.c0(Rgbw::BLUE),
                    4 => s.c0(Rgbw::VIOLET),
                    5 => s.c0(Rgbw::MAGENTA),
                    _ => {},
                },
                (_, 2) => match x {
                    2 => s.c0(Rgbw::RED),
                    3 => s.c0(Rgbw::ORANGE),
                    4 => s.c0(Rgbw::YELLOW),
                    5 => s.c0(Rgbw::LIME),
                    _ => {},
                },
                (_, 3) => match x {
                    2 => s.c1(Rgbw::BLACK),
                    3 => s.c1(Rgbw::WHITE),
                    4 => s.c1 = Op::f(|s| Rgbw::hsv(s.phi(16, 1), 1.0, 1.0)),
                    5 => s.c0(Rgbw::WHITE),
                    _ => {},
                },
                (_, 4) => match x {
                    2 => s.c1(Rgbw::CYAN),
                    3 => s.c1(Rgbw::BLUE),
                    4 => s.c1(Rgbw::VIOLET),
                    5 => s.c1(Rgbw::MAGENTA),
                    _ => {},
                },
                (_, 5) => match x {
                    2 => s.c1(Rgbw::RED),
                    3 => s.c1(Rgbw::ORANGE),
                    4 => s.c1(Rgbw::YELLOW),
                    5 => s.c1(Rgbw::LIME),
                    _ => {},
                },
                (_, 6) => match x {
                    2 => s.cc(Rgbw::BLACK, Rgbw::BLACK),
                    3 => s.cc(Rgbw::WHITE, Rgbw::WHITE),
                    4 => s.ccc(|s| Rgbw::hsv(s.phi(16, 1), 1.0, 1.0)),
                    5 => s.c0(Rgbw::VIOLET),
                    _ => {},
                },
                _ => {},
            }
        }

        // holds
        match (x, y) {
            // hold pressure env
            // (6, 2) => s.beat0 = Beat::Fr(fr.in_exp()),
            // (7, 2) => s.beat0 = Beat::Fr(fr.in_exp()),
            (0, 7) => s.mode1 = Mode::Press { fr: fr.in_exp() },
            (7, 7) => s.mode0 = Mode::Press { fr: fr.in_exp() },

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

            _ => {},
        }

        // resets
        if b {
            match (x, y) {
                (1, 0..=2) | (6, 0..=2) => l.beam_pos = BeamPos::Down,
                (1, 3..=4) | (6, 3..=4) => l.beam_pos = BeamPos::Square { pd: Pd(1, 1) },
                (1, 5..=6) | (6, 5..=6) => l.beam_pos = BeamPos::Out,
                _ => {},
            }

            match (x, y) {
                (1, 0..=2) | (6, 0..=2) => l.spider_pos = SpiderPos::Down,
                (1, 3..=4) | (6, 3..=4) => l.spider_pos = SpiderPos::Alternate { pd: Pd(4, 1) },
                (1, 5..=6) | (6, 5..=6) => l.spider_pos = SpiderPos::Up,
                _ => {},
            }
        }
    }
}

///////////////////////// CTRL /////////////////////////

fn on_ctrl(
    event: launch_control_xl::Input,
    s: &mut State,
    l: &mut Lights,
    ctrl: &mut Midi<LaunchControlXL>,
) {
    use launch_control_xl::{types::*, *};

    println!("{event:?}");

    match event {
        // time mod knob
        Input::SendA(0, fr) => s.t_mod = fr.map(-1.0..1.0, 0.5..2.0),
        // phi mod select buttons
        Input::Mute(true) => s.phi_mod = 0.5,
        Input::Solo(true) => s.phi_mod = 1.0,
        Input::Record(true) => s.phi_mod = 2.0,
        // alpha slider
        Input::Slider(0, fr) => s.alpha = fr,
        // Input::Slider(0, range) => s.range = range,
        Input::Slider(1, fr) => s.min = fr,
        Input::Slider(2, fr) => s.max = fr,

        // beam patterns
        Input::Control(i, true) => l.beam_pos = match i {
            1 => BeamPos::SpreadOut,
            2 => BeamPos::SpreadIn,
            3 => BeamPos::Cross,
            4 => BeamPos::CrissCross,
            6 => BeamPos::WaveY { pd: Pd(2, 1) },
            7 => BeamPos::Square { pd: Pd(2, 1) },
            _ => BeamPos::Down,
        },

        // laser tweaks
        Input::Focus(0, true) => l.laser.on = !l.laser.on,
        Input::Slider(0, fr) => l.laser.pattern = LaserPattern::Raw(fr.byte()),
        Input::Slider(1, fr) => l.laser.rotate = fr,
        Input::Slider(2, fr) => l.laser.xflip = fr,
        Input::Slider(3, fr) => l.laser.yflip = fr,
        Input::Slider(4, fr) => l.laser.x = fr,
        Input::Slider(5, fr) => l.laser.y = fr,
        Input::Slider(6, fr) => l.laser.size = fr,
        Input::Slider(7, fr) => l.laser.color = LaserColor::Raw(fr.byte()),

        _ => {},
    }
}

///////////////////////// IO /////////////////////////

fn io(
    ctx: &egui::Context,
    dt: f64, s: &mut State,
    l: &mut Lights,
    pad: &mut Option<Midi<LaunchpadX>>,
    ctrl: &mut Option<Midi<LaunchControlXL>>,
    e131: &mut E131
) {
    if let Some(pad) = pad.as_mut() {
        for event in pad.recv() {
            on_pad(event, s, l, pad);
        }
    }
    if let Some(ctrl) = ctrl.as_mut() {
        for event in ctrl.recv() {
            on_ctrl(event, s, l, ctrl);
        }
    }

    // real time
    s.dt0 = dt;
    s.t0 += s.dt0;
    s.phi0 = (s.phi0 + (s.dt0 * (s.bpm / 60.0))).fmod(16.0);
    // modified time
    s.dt = s.dt0 * s.t_mod;
    s.t += s.dt;
    s.phi = (s.phi + (s.dt * (s.bpm / 60.0))).fmod(16.0);

    // tick alpha up/down
    // s.alpha = if s.alpha_up { (s.alpha + s.dt(1, 1)) } else { s.alpha };
    // s.alpha = if s.alpha_down { (s.alpha - s.dt(1, 1)).max(0.0) } else { s.alpha };

    // apply hold fallbacks
    // let env0 = s.env_h.or(&s.env_h0.or(&s.env0));
    // let env1 = s.env_h.or(&s.env_h1.or(&s.env1));
    // let c0 = s.c_h.or(&s.c_h0.or(&s.c0));
    // let c1 = s.c_h.or(&s.c_h1.or(&s.c1));

    // global alpha
    // let a = s.alpha;

    // envelope alpha
    // let env0 = env0(s);
    // let env1 = env1(s);
    // let a0 = (a * s.beat0.or(s, env0)).min(1.0);
    // let a1 = (a * s.beat1.or(s, env1)).min(1.0);

    // alpha adjust colors
    // let c0 = c0(s).a(a0);
    // let c1 = c1(s).a(a1);

    let env0 = (s.mode0.env(s) * s.alpha).max(s.min).min(s.max);
    let env1 = (s.mode1.env(s) * s.alpha).max(s.min).min(s.max);

    let c00 = (s.c0.clone())(s);
    let c11 = (s.c1.clone())(s);

    let c0 = c00.a(env0);
    let c1 = c11.a(env1);

    // pad stuff
    if let Some(pad) = pad.as_mut() {
        use launchpad_x::{types::*, *};
        use self::Mode;

        let mut batch: Vec<(Pos, Color)> = vec![];

        let mut rgb = |x, y, Rgb(r, g, b): Rgb| batch.push((Coord(x,y).into(), Color::Rgb(r, g, b)));

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
            rgb(2, 3*i+2, Rgb::RED);
            rgb(3, 3*i+2, Rgb::ORANGE);
            rgb(4, 3*i+2, Rgb::YELLOW);
            rgb(5, 3*i+2, Rgb::LIME);
            rgb(2, 3*i+1, Rgb::CYAN);
            rgb(3, 3*i+1, Rgb::BLUE);
            rgb(4, 3*i+1, Rgb::VIOLET);
            rgb(5, 3*i+1, Rgb::MAGENTA);
            rgb(2, 3*i, Rgb::BLACK);
            rgb(3, 3*i, Rgb::WHITE);
            rgb(4, 3*i, Rgb::hsv(s.phi(16, 1), 1.0, 1.0));
            rgb(5, 3*i, Rgb::WHITE);
        }

        // basic modes
        for (i, c) in [(0, c11), (1, c00)] {
            rgb(1+5*i, 0, Rgb::BLACK);
            rgb(1+5*i, 1, c.into());
            rgb(1+5*i, 2, c.a(Mode::Hover.env(s)).into());
        }

        rgb(1, 3, c11.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
        rgb(1, 4, c11.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());
        rgb(6, 3, c00.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.2).into() }.env(s)).into());
        rgb(6, 4, c00.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.2).into() }.env(s)).into());

        rgb(0, 5, c11.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
        rgb(0, 6, c11.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());
        rgb(7, 5, c00.a(Mode::AutoBeat { pd: Pd(2, 1), r: (1.0..0.0).into() }.env(s)).into());
        rgb(7, 6, c00.a(Mode::AutoBeat { pd: Pd(1, 1), r: (1.0..0.0).into() }.env(s)).into());

        // beats
        for i in 0..=4 {
            rgb(0, i, Rgb::WHITE);
            rgb(7, i, Rgb::WHITE);
        }


        if let Mode::Press { fr } = s.mode0 {
            rgb(7, 7, c0.into());
        }
        if let Mode::Press { fr } = s.mode1 {
            rgb(0, 7, c1.into());
        }

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
        rgb(8, 8, match s.pd(Pd(1, 1)).bsquare(1.0, 0.1) {
            true => match s.pd(Pd(4, 1)).bsquare(1.0, 0.2) {
                true => Rgb::VIOLET,
                false => Rgb::WHITE,
            },
            false => Rgb::BLACK,
        });

        pad.send(Output::Batch(batch));
    }

    // lights
    // l.par_src = s.par_src_h.or(&s.par_src);
    // l.strobe_src = s.strobe_src_h.or(&s.strobe_src);
    l.tick(s, c0, c1);

    for beam in &mut l.beams {
        beam.pitch = 1.0 - beam.pitch;
    }    

    l.send(e131);

    egui::CentralPanel::default().show(ctx, |ui| {
        let size = ui.available_size();
        let (resp, painter) = ui.allocate_painter(size, egui::Sense::hover());
        l.paint(&painter, size.x as f64, size.y as f64);
    });
}

///////////////////////// HELPERS /////////////////////////

impl State {
    fn phi(&self, n: usize, d: usize) -> f64 { self.pd(Pd(n, d)) }
    fn pd(&self, pd: Pd) -> f64 { self.phi.mod_div(pd.fr() * self.phi_mod) }

    fn dt(&self, n: usize, d: usize) -> f64 { self.dt / ((self.bpm / 60.0) * Pd(n, d).fr()) }

    fn c(&mut self, c: Rgbw) { self.cc(c, c); }
    fn cc(&mut self, c0: Rgbw, c1: Rgbw) { self.c0 = Op::v(c0); self.c1 = Op::v(c1); }
    fn c0(&mut self, c0: Rgbw) { self.c0 = Op::v(c0); }
    fn c1(&mut self, c1: Rgbw) { self.c1 = Op::v(c1); }
    fn ccc(&mut self, f: impl Fn(&mut State) -> Rgbw + 'static) { self.c0 = Op::f(f); self.c1 = self.c0.clone(); }
}

impl Mode {
    fn env(self, s: &State) -> f64 {
        match self {
            Mode::Off => 0.0,
            Mode::On => 1.0,
            Mode::Hover => s.phi(8, 1).ssin(1.0).map(-1.0..1.0, 0.3..0.7),
            Mode::AutoBeat { pd, r } => s.pd(pd).ramp(1.0).lerp(r).in_quad(),
            Mode::Beat { t, pd, r } => {
                let dt = s.t - t;
                let len = (60.0 / s.bpm) * pd.fr();

                if dt >= len {
                    r.hi
                } else {
                    (dt / len).ramp(1.0).lerp(r).in_quad()
                }
            },
            Mode::Press { fr } => fr,
            Mode::Strobe { pd, duty } => s.pd(pd).square(1.0, duty.in_exp().lerp(1.0..0.5)),
        }
    }
}

///////////////////////// MAIN /////////////////////////

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let mut e131 = E131::new("10.16.4.1".parse()?, E131_PORT, 1)?;

    let mut pad = Midi::connect("Launchpad X:Launchpad X LPX MIDI", LaunchpadX::default()).ok();
    let mut ctrl = Midi::connect("Launch Control XL:Launch Control XL", LaunchControlXL).ok();

    let mut s = State::default();
    let mut l = Lights::default();

    if let Some(pad) = pad.as_mut() {
        use launchpad_x::{types::*, *};
        pad.send(Output::Pressure(Pressure::Polyphonic, PressureCurve::Medium));
        pad.send(Output::Brightness(1.0));
    }

    // defaults
    s.bpm = 120.0;
    s.alpha = 0.0;
    s.t_mod = 1.0;
    s.phi_mod = 1.0;
    // s.ccc(|s| Rgbw::hsv(s.phi(16, 1), 1.0, 1.0));
    s.c(Rgbw::WHITE);
    s.max = 1.0;
    // s.c0 = Op::v(Rgbw::VIOLET);
    // s.c1 = Op::v(Rgbw::WHITE);

    s.strobe_src = Source::C0;

    // l.par_src = Source::Chase { pd: Pd(1, 1), duty: 0.2 };
    l.spider_src = Source::SpiderBoth;
    l.beam_src = Source::C1;
    l.bar_src = Source::C1;

    let mut last = Instant::now();

    // let mut time = 0.0;
    // let mut red = 1.0;

    eframe::run_simple_native("mslive2023", Default::default(), move |ctx, _frame| {
        // render at at 200fps
        if last.elapsed() > Duration::from_millis(5) {
            let dt = last.elapsed().as_secs_f64();
            last = Instant::now();

            // time += dt;

            // if let Some(pad) = pad.as_mut() {
            //     for event in pad.recv() {
            //         use launchpad_x::Input;
            //         use launchpad_x::types::{Index, Coord, Pos};
            //         match event {
            //             Input::Press(index, _) => {
            //                 println!("{event:?}");
            //                 let pos: Pos = index.into();
            //                 let Coord(x, y) = pos.into();

            //                 match (x, y) {
            //                     (0, 0) => {
            //                         red = 1.0;
            //                     },
            //                     (0, 1) => {
            //                         red = 0.0;
            //                     },
            //                     _ => {},
            //                 }
            //             },
            //             _ => {},
            //         }
            //     }
            // }

            // for beam in &mut l.beams {
            //     beam.speed = 1.0;
            //     // beam.yaw = (time * 0.25).sin() * 0.5 + 0.5;
            //     beam.color.0 = (time).sin() * 0.5 + 0.5;
            // }

            // println!("time={time:?}");
            // for beam in &mut l.beams {
            //     beam.color.0 = red * (time / 4.0).fmod(1.0).in_quad();
            // }
            // l.send(&mut e131);

            io(ctx, dt, &mut s, &mut l, &mut pad, &mut ctrl, &mut e131);

            // println!("{:?}", &l.bars);
        }

        // no damage tracking
        ctx.request_repaint();
    });

    Ok(())
}
