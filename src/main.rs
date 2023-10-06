#![allow(unused)]
#![allow(clippy::single_match)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::needless_pass_by_ref_mut)]

#![feature(stmt_expr_attributes)]

use color_eyre::Result;
use itertools::Itertools;
use stagebridge::color::Rgbw;
use stagebridge::dmx::device::laser_scan_30w::{LaserPattern, LaserColor};
use std::time::Instant;
use std::{thread, time::Duration};

use stagebridge::e131::{E131, E131_PORT};
use stagebridge::midi::device::{
    launch_control_xl::{self, LaunchControlXL},
    launchpad_x::{self, LaunchpadX},
};
use stagebridge::midi::Midi;
use stagebridge::num::{Float, Ease};

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

    // special case modes
    special: Special,

    // envelopes
    env0: Op<f64>,
    env1: Op<f64>,
    env_h: Hold<Op<f64>>,
    env_h0: Hold<Op<f64>>,
    env_h1: Hold<Op<f64>>,

    // color
    c0: Op<Rgbw>,
    c1: Op<Rgbw>,
    c_h: Hold<Op<Rgbw>>,
    c_h0: Hold<Op<Rgbw>>,
    c_h1: Hold<Op<Rgbw>>,

    // manual beats
    beat0: Beat,
    beat1: Beat,

    off: bool,
    alpha: f64,
    alpha_up: bool,
    alpha_down: bool,
}

#[derive(Clone, Default)]
enum Special {
    #[default]
    None,
    // Chase { order: Vec<usize> },
}

///////////////////////// PAD /////////////////////////

fn on_pad(
    event: launchpad_x::Input,
    s: &mut State,
    l: &mut Lights,
    pad: &mut Midi<LaunchpadX>,
    ctrl: &mut Midi<LaunchControlXL>,
) {
    use launchpad_x::{types::*, *};

    match event {
        // alpha up/down hold
        Input::Up(b) => s.alpha_up = b,
        Input::Down(b) => s.alpha_down = b,
        Input::Left(true) => s.alpha = 1.0,
        _ => {},
    }

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
                (5, 7) => s.beatmatch.push(s.t0),
                // reset phase
                (6, 7) => s.phi = 0.0,
                // apply beatmatch
                (7, 7) => match s.beatmatch.len() {
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

                // slow envs
                (3, 0) => s.env(|_| 0.1),
                (3, 1) => {},
                (3, 2) => s.env(|s| s.phi(1, 1).ramp(1.0).inv().lerp(0.2..0.3)),

                // fast envs
                (4, 0) => s.env(|_| 0.4),
                (4, 1) => {},
                (4, 2) => s.env(|s| s.phi(1, 1).ramp(1.0).inv().lerp(0.2..0.5)),

                // mod colors
                (0, 0) => s.c(Rgbw::BLACK),
                (0, 1) => s.ccc(|s| Rgbw::hsv(s.phi(16, 1), 1.0, 1.0)),
                (0, 2) => s.c(Rgbw::WHITE),

                // blue/green colors
                (1, 0) => s.c(Rgbw::BLUE),
                (1, 1) => s.cc(Rgbw::PBLUE, Rgbw::PBLUE),
                (1, 2) => s.cc(Rgbw::CYAN, Rgbw::MINT),
                (1, 3) => s.cc(Rgbw::LIME, Rgbw::LIME),

                // red/blue colors
                (2, 0) => s.c(Rgbw::RED),
                (2, 1) => s.cc(Rgbw::RED, Rgbw::BLUE),
                (2, 2) => s.cc(Rgbw::RED, Rgbw::MAGENTA),
                (2, 3) => s.cc(Rgbw::MAGENTA, Rgbw::RED),
                (2, 4) => s.c(Rgbw::VIOLET),

                // env0 beat
                (6, 0) => s.beat0 = Beat::at(s, Pd(2, 1), 0.0..1.0),
                (7, 0) => s.beat1 = Beat::at(s, Pd(2, 1), 0.0..1.0),

                // env1 beat
                (6, 1) => s.beat0 = Beat::at(s, Pd(1, 1), 0.0..1.0),
                (7, 1) => s.beat1 = Beat::at(s, Pd(1, 1), 0.0..1.0),
                _ => {},
            }
        }

        // holds
        match (x, y) {
            // hold mod colors
            (5, 1) => {
                s.c_h.hold(x, y, b, Op::f(|s| Rgbw::hsv(s.pd(Pd(4, 1)), 1.0, 1.0)));
                s.env_h.hold(x, y, b, Op::v(1.0));
            },
            (5, 2) => s.c_h.hold(x, y, b, Op::v(Rgbw::BLACK)),
            (5, 3) => {
                s.c_h.hold(x, y, b, Op::v(Rgbw::WHITE));
                s.env_h.hold(x, y, b, Op::v(1.0));
            },

            // hold strobe w/ pressure
            (6, 3) => s.env_h.hold(x, y, b, Op::f(move |s| s.pd(Pd(1, 4)).square(1.0, fr.in_exp().lerp(1.0..0.5)))),
            (7, 3) => s.env_h.hold(x, y, b, Op::f(move |s| s.pd(Pd(1, 8)).square(1.0, fr.in_exp().lerp(1.0..0.5)))),
            _ => {},
        }
    }
}

///////////////////////// CTRL /////////////////////////

fn on_ctrl(
    event: launch_control_xl::Input,
    s: &mut State,
    l: &mut Lights,
    pad: &mut Midi<LaunchpadX>,
    ctrl: &mut Midi<LaunchControlXL>,
) {
    use launch_control_xl::{types::*, *};

    match event {
        // time mod knob
        Input::SendA(0, fr) => s.t_mod = fr.map(-1.0..1.0, 0.5..2.0),
        // phi mod select buttons
        Input::Mute(true) => s.phi_mod = 0.5,
        Input::Solo(true) => s.phi_mod = 1.0,
        Input::Record(true) => s.phi_mod = 2.0,
        // alpha slider
        Input::Slider(0, fr) => s.alpha = fr,

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

fn io(ctx: &egui::Context, dt: f64, s: &mut State, l: &mut Lights, pad: &mut Midi<LaunchpadX>, ctrl: &mut Midi<LaunchControlXL>, e131: &mut E131) {
    for event in pad.recv() {
        on_pad(event, s, l, pad, ctrl);
    }
    for event in ctrl.recv() {
        on_ctrl(event, s, l, pad, ctrl);
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
    s.alpha = if s.alpha_up { (s.alpha + s.dt(1, 1)).min(1.0) } else { s.alpha };
    s.alpha = if s.alpha_down { (s.alpha - s.dt(1, 1)).max(0.0) } else { s.alpha };

    // apply hold fallbacks
    let env0 = s.env_h.or(&s.env_h0.or(&s.env0));
    let env1 = s.env_h.or(&s.env_h1.or(&s.env1));
    let c0 = s.c_h.or(&s.c_h0.or(&s.c0));
    let c1 = s.c_h.or(&s.c_h1.or(&s.c1));

    // global alpha
    let a = s.alpha;

    // envelope alpha
    let env0 = env0(s);
    let env1 = env1(s);
    let a0 = a * s.beat0.or(s, env0);
    let a1 = a * s.beat1.or(s, env1);

    // alpha adjust colors
    let c0 = c0(s).a(a0);
    let c1 = c1(s).a(a1);

    // pad stuff
    {
        use launchpad_x::{types::*, *};

        // main grid c0
        pad.send(Output::ClearColor(c0.into()));

        // outer ring c1
        pad.send(Output::Batch(
            (0..8)
                .flat_map(|i| [Coord(i, 8), Coord(8, i)])
                .map(|coord| (coord.into(), c1.into()))
                .collect(),
        ));

        // beat, phi=0 indicator
        pad.send(Output::Light(
            Coord(8, 8).into(),
            match s.pd(Pd(1, 1)).bsquare(1.0, 0.1) {
                true => match s.pd(Pd(4, 1)).bsquare(1.0, 0.2) {
                    true => PaletteColor::Violet,
                    false => PaletteColor::White,
                },
                false => PaletteColor::Off,
            },
        ))
    }

    l.tick(s, c0, c1);
    l.send(e131);

    egui::CentralPanel::default().show(ctx, |ui| {
        let (resp, p) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());

        // add colored rect shape based on c0
        p.add(egui::Shape::circle_filled(egui::Pos2::new(100.0, 100.0), 100.0, c0.e()));
    });
}

///////////////////////// HELPERS /////////////////////////

impl State {
    fn phi(&self, n: usize, d: usize) -> f64 { self.pd(Pd(n, d)) }
    fn pd(&self, pd: Pd) -> f64 { self.phi.mod_div(pd.fr() * self.phi_mod) }

    fn dt(&self, n: usize, d: usize) -> f64 { self.dt / ((self.bpm / 60.0) * Pd(n, d).fr()) }

    fn c(&mut self, c: Rgbw) { self.cc(c, c); }
    fn cc(&mut self, c0: Rgbw, c1: Rgbw) { self.c0 = Op::v(c0); self.c1 = Op::v(c1); }
    fn ccc(&mut self, f: impl Fn(&mut State) -> Rgbw + 'static) { self.c0 = Op::f(f); self.c1 = self.c0.clone(); }

    fn env(&mut self, f: impl Fn(&mut State) -> f64 + 'static) {
        self.beat0 = Beat::Off;
        self.beat1 = Beat::Off;

        self.env0 = f.into();
        self.env1 = self.env0.clone();
    }
}

///////////////////////// MAIN /////////////////////////

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let mut e131 = E131::new("10.16.4.1".parse()?, E131_PORT, 1)?;

    let mut pad = Midi::connect("Launchpad X:Launchpad X LPX MIDI", LaunchpadX::default())?;
    let mut ctrl = Midi::connect("Launch Control XL:Launch Control XL", LaunchControlXL)?;

    let mut s = State::default();
    let mut l = Lights::default();

    // defaults
    s.bpm = 120.0;
    s.alpha = 1.0;
    s.t_mod = 1.0;
    s.phi_mod = 1.0;
    s.c0 = Op::v(Rgbw::WHITE);
    s.c1 = Op::v(Rgbw::VIOLET);
    s.env0 = Op::v(1.0);
    s.env1 = Op::v(1.0);

    let mut last = Instant::now();

    eframe::run_simple_native("mslive2023", Default::default(), move |ctx, _frame| {
        // render at at 200fps
        if last.elapsed() > Duration::from_millis(5) {
            let dt = last.elapsed().as_secs_f64();
            last = Instant::now();

            io(ctx, dt, &mut s, &mut l, &mut pad, &mut ctrl, &mut e131);
        }

        // no damage tracking
        ctx.request_repaint();
    });

    Ok(())
}
