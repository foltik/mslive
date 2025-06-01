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
    /// Brightness is mapped onto this range
    pub range: f64,
    pub dimmer_brightness: f64,
    /// Global speed modifiers
    pub speed_coarse: f64,
    pub speed_fine: f64,
    /// Global duty cycle modifications
    pub duty0: f64,
    pub duty1: f64,
    pub duty2: f64,
    /// Generic modifiers depending on patterns
    pub sliders: [f64; 8],
    pub knobs: [f64; 8],

    /* Patterns */
    pub color0: Color,
    pub color1: Color,
    pub pattern0: Pattern,
    pub pattern1: Pattern,

    /* Jam */
    pub jam_rgbw0: [f64; 9],
    pub jam_rgbw1: [f64; 9],
    pub jam_dimmer: [f64; 9],
}

impl State {
    pub fn new() -> Self {
        Self {
            brightness: 1.0,
            dimmer_brightness: 1.0,
            bpm: 120.0,
            phi_mul: 1.0,
            speed_coarse: 1.0,
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
    Jam,
    Todo,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Pattern {
    #[default]
    Unit,
    Sine,
    Ramp,
}

impl Pattern {
    pub fn apply(self, s: &State, duty: f64) -> f64 {
        match self {
            Pattern::Unit => 1.0,
            Pattern::Sine => s.t.fsin(duty),
            Pattern::Ramp => s.t.ramp(duty),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Color {
    /// House lights color
    #[default]
    House,

    // Pure colors
    White,
    Red,
    Orange,
    Yellow,
    Green,
    Pea,
    Blue,
    Cyan,
    Purple,
    Magenta,
    Mint,

    // Split pure plus white
    RedWhite,
    GreenWhite,
    BlueWhite,

    // Periodic sine hue shift between two colors
    RedPurpleShift,
    GreenYellowShift,
    BlueGreenShift,

    // Rotating hsv
    Rainbow,
    // ManualHue,
    // ManualRgbw,
}

///////////////////////// LIGHTS /////////////////////////

pub fn render_lights(s: &mut State, l: &mut Lights) {
    /* House lights override */
    if s.house > 0.0 {
        for i in 0..9 {
            l.rgbw[i] = Color::House.render(s) * s.house;
        }
        l.send();
        return;
    }

    match s.mode {
        Mode::Jam => {
            let color0 = s.color0.render(s);
            let color1 = s.color1.render(s);
            for i in 0..9 {
                let fr0 = s.jam_rgbw0[i].max(0.0);
                let fr1 = s.jam_rgbw1[i].max(0.0);

                if fr0 > 0.0 {
                    l.rgbw[i] = color0 * fr0 * s.brightness * s.range;
                } else if fr1 > 0.0 {
                    l.rgbw[i] = color1 * fr1 * s.brightness * s.range;
                }
            }
            for i in 0..9 {
                l.dimmer[i] = s.jam_dimmer[i].max(0.0) * s.dimmer_brightness;
            }
        }
        Mode::Todo => {
            // TODO
        }
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

    match s.mode {
        Mode::Todo => {
            // Beatmatch buttons
            set(0, 7, Rgb::VIOLET);
            set(7, 7, Rgb::VIOLET);
        }
        Mode::Jam => {
            use crate::logic::Color;

            for x in 0..3 {
                for y in 0..3 {
                    let n = (y * 3) + x;
                    set(x, y, (Rgbw::WHITE * s.jam_rgbw0[n as usize]).into());
                    set(x + 5, y, (Rgbw::WHITE * s.jam_rgbw1[n as usize]).into());
                }
            }

            for i in 0..3 {
                set(3, i, s.color0.render(s).into());
                set(4, i, s.color1.render(s).into());
            }

            set(0, 8, Rgb::WHITE * if s.pattern0 == Pattern::Unit { 1.0 } else { 0.0 });
            set(1, 8, Rgb::WHITE * if s.pattern0 == Pattern::Sine { 1.0 } else { 0.0 });
            set(2, 8, Rgb::WHITE * if s.pattern0 == Pattern::Ramp { 1.0 } else { 0.0 });
            set(5, 8, Rgb::WHITE * if s.pattern1 == Pattern::Unit { 1.0 } else { 0.0 });
            set(6, 8, Rgb::WHITE * if s.pattern1 == Pattern::Sine { 1.0 } else { 0.0 });
            set(7, 8, Rgb::WHITE * if s.pattern1 == Pattern::Ramp { 1.0 } else { 0.0 });

            set(0, 7, Color::RedPurpleShift.render(s).into());
            set(1, 7, Color::GreenYellowShift.render(s).into());
            set(2, 7, Color::BlueGreenShift.render(s).into());

            set(0, 6, Color::Red.render(s).into());
            set(1, 6, Color::Orange.render(s).into());
            set(2, 6, Color::Yellow.render(s).into());

            set(0, 5, Color::Green.render(s).into());
            set(1, 5, Color::Mint.render(s).into());
            set(2, 5, Color::Cyan.render(s).into());

            set(0, 4, Color::Blue.render(s).into());
            set(1, 4, Color::Purple.render(s).into());
            set(2, 4, Color::Magenta.render(s).into());

            set(0, 3, Color::White.render(s).into());
            set(1, 3, Color::Rainbow.render(s).into());

            /**/

            set(5, 7, Color::RedPurpleShift.render(s).into());
            set(6, 7, Color::GreenYellowShift.render(s).into());
            set(7, 7, Color::BlueGreenShift.render(s).into());

            set(5, 6, Color::Red.render(s).into());
            set(6, 6, Color::Orange.render(s).into());
            set(7, 6, Color::Yellow.render(s).into());

            set(5, 5, Color::Green.render(s).into());
            set(6, 5, Color::Mint.render(s).into());
            set(7, 5, Color::Cyan.render(s).into());

            set(5, 4, Color::Blue.render(s).into());
            set(6, 4, Color::Purple.render(s).into());
            set(7, 4, Color::Magenta.render(s).into());

            set(5, 3, Color::White.render(s).into());
            set(6, 3, Color::Rainbow.render(s).into());

            set(3, 3, (Rgbw::WHITE * s.jam_dimmer[0]).into());
            set(4, 3, (Rgbw::WHITE * s.jam_dimmer[1]).into());
            set(3, 4, (Rgbw::WHITE * s.jam_dimmer[2]).into());
            set(4, 4, (Rgbw::WHITE * s.jam_dimmer[3]).into());
            set(3, 5, (Rgbw::WHITE * s.jam_dimmer[4]).into());
            set(4, 5, (Rgbw::WHITE * s.jam_dimmer[5]).into());
            set(3, 6, (Rgbw::WHITE * s.jam_dimmer[6]).into());
            set(4, 6, (Rgbw::WHITE * s.jam_dimmer[7]).into());
        }
    }

    pad.send(Output::Batch(batch));
}

///////////////////////// CTRL /////////////////////////

#[rustfmt::skip]
pub fn render_ctrl(s: &mut State, ctrl: &mut Midi<LaunchControlXL>) {
    use launch_control_xl::{types::*, *};
    use crate::logic::Mode as Mode;
    let mut set = |i, b| ctrl.send(Output::Control(i, Color::Red, if b { Brightness::High } else { Brightness::Off }));
}

///////////////////////// TICK /////////////////////////

pub fn tick(dt: f64, s: &mut State, l: &mut Lights) {
    s.dt = dt;
    s.t0 += dt;
    s.t += dt * s.speed_coarse;
    s.phi = (s.phi + (dt * (s.bpm / 60.0) * s.phi_mul)).fmod(64.0);

    match s.mode {
        Mode::Todo => {}
        Mode::Jam => {
            for i in 0..9 {
                let map = |duty: f64| if duty == 1.0 { 4.0 } else { duty };
                s.jam_rgbw0[i] -= s.dt * map(1.0 - s.duty0) * s.pattern0.apply(s, s.duty0) * 4.0;
                s.jam_rgbw1[i] -= s.dt * map(1.0 - s.duty1) * s.pattern1.apply(s, s.duty1) * 4.0;
                s.jam_dimmer[i] -= s.dt * (1.0 - s.duty2) * 2.0;
            }
        }
    }
}

///////////////////////// PAD INPUT /////////////////////////

#[rustfmt::skip]
pub fn on_pad(s: &mut State, l: &mut Lights, pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
    use crate::logic::Mode;
    use launchpad_x::{types::*, *};
    log::debug!("pad: {event:?}");

    match event {
        Input::Up(true) => s.mode = Mode::Todo,
        Input::Down(true) => s.mode = Mode::Jam,
        _ => {},
    }

    if let Some((x, y)) = match event {
        Input::Press(i, _) => Some((Coord::from(i).0, Coord::from(i).1)),
        _ => None,
    } {

        match s.mode {
            Mode::Todo => {},
            Mode::Jam => {
                use crate::logic::Color;
                match (x, y) {

                    (3, 3) => s.jam_dimmer[0] = 1.0,
                    (4, 3) => s.jam_dimmer[1] = 1.0,
                    (3, 4) => s.jam_dimmer[2] = 1.0,
                    (4, 4) => s.jam_dimmer[3] = 1.0,
                    (3, 5) => s.jam_dimmer[4] = 1.0,
                    (4, 5) => s.jam_dimmer[5] = 1.0,
                    (3, 6) => s.jam_dimmer[6] = 1.0,
                    (4, 6) => s.jam_dimmer[7] = 1.0,

                    (0..3, 0..3) => {
                        let n = (y * 3) + x;
                        s.jam_rgbw0[n as usize] = 1.0;
                        s.jam_rgbw1[n as usize] = 0.0;
                    },
                    (5..8, 0..3) => {
                        let n = (y * 3) + (x - 5);
                        s.jam_rgbw0[n as usize] = 0.0;
                        s.jam_rgbw1[n as usize] = 1.0;
                    },
                    (2, 3) => {
                        for i in 0..9 {
                            s.jam_rgbw0[i] = 1.0;
                            s.jam_rgbw1[i] = 0.0;
                        }
                    },
                    (7, 3) => {
                        for i in 0..9 {
                            s.jam_rgbw0[i] = 0.0;
                            s.jam_rgbw1[i] = 1.0;
                        }
                    },
                    (0, 8) => s.pattern0 = Pattern::Unit,
                    (1, 8) => s.pattern0 = Pattern::Sine,
                    (2, 8) => s.pattern0 = Pattern::Ramp,

                    (5, 8) => s.pattern1 = Pattern::Unit,
                    (6, 8) => s.pattern1 = Pattern::Sine,
                    (7, 8) => s.pattern1 = Pattern::Ramp,
                    _ => {
                        s.color0 = match (x, y) {
                            (0, 7) => Color::RedPurpleShift,
                            (1, 7) => Color::GreenYellowShift,
                            (2, 7) => Color::BlueGreenShift,

                            (0, 6) => Color::Red,
                            (1, 6) => Color::Orange,
                            (2, 6) => Color::Yellow,

                            (0, 5) => Color::Green,
                            (1, 5) => Color::Mint,
                            (2, 5) => Color::Cyan,

                            (0, 4) => Color::Blue,
                            (1, 4) => Color::Purple,
                            (2, 4) => Color::Magenta,

                            (0, 3) => Color::White,
                            (1, 3) => Color::Rainbow,

                            _ => s.color0,
                        };

                        s.color1 = match (x, y) {
                            (5, 7) => Color::RedPurpleShift,
                            (6, 7) => Color::GreenYellowShift,
                            (7, 7) => Color::BlueGreenShift,

                            (5, 6) => Color::Red,
                            (6, 6) => Color::Orange,
                            (7, 6) => Color::Yellow,

                            (5, 5) => Color::Green,
                            (6, 5) => Color::Mint,
                            (7, 5) => Color::Cyan,

                            (5, 4) => Color::Blue,
                            (6, 4) => Color::Purple,
                            (7, 4) => Color::Magenta,

                            (5, 3) => Color::White,
                            (6, 3) => Color::Rainbow,

                            _ => s.color1,
                        };
                    }
                }
            },
        }

        match (x, y) {
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
        // Input::SendA(0, fr) => s.brightness = (fr * 0.5) + 0.5,
        Input::SendA(0, fr) => s.speed_coarse = (fr * 0.5) + 0.5,
        Input::SendA(1, fr) => s.speed_fine = (fr * 0.5) + 0.5,
        Input::SendA(7, fr) => s.house = (fr * 0.5) + 0.5,

        Input::Pan(i, fr) => s.knobs[i as usize] = (fr * 0.5) + 0.5,

        Input::Slider(0, fr) => s.brightness = fr,
        Input::Slider(1, fr) => s.dimmer_brightness = fr,
        Input::Slider(2, fr) => s.duty0 = fr,
        Input::Slider(3, fr) => s.duty1 = fr,
        Input::Slider(4, fr) => s.duty2 = fr,

        Input::Slider(7, fr) => s.range = fr,
        _ => {}
    }
}

impl Color {
    pub fn render(self, s: &State) -> Rgbw {
        use Color::*;

        match self {
            /* ---------- Static colours ---------- */
            House => Rgbw(1.00, 0.48, 0.0, 0.0),
            White => Rgbw::WHITE,
            Red => Rgbw::RED,
            Orange => Rgbw::ORANGE,
            Yellow => Rgbw::YELLOW,
            Green => Rgbw::LIME,
            Pea => Rgbw::PEA,
            Blue => Rgbw::BLUE,
            Cyan => Rgbw::CYAN,
            Purple => Rgbw::VIOLET,
            Magenta => Rgbw::MAGENTA,
            Mint => Rgbw::MINT,

            RedWhite => Rgbw(1.0, 0.0, 0.0, 1.0),
            GreenWhite => Rgbw(0.0, 1.0, 0.0, 1.0),
            BlueWhite => Rgbw(0.0, 0.0, 1.0, 1.0),

            RedPurpleShift => {
                let t = ((s.t * 0.25).sin() * 0.5 + 0.5); // 0‥1
                let a = Rgbw::RED;
                let b = Rgbw::VIOLET;
                Rgbw(t.lerp(a.0..b.0), t.lerp(a.1..b.1), t.lerp(a.2..b.2), t.lerp(a.3..b.3))
            }
            GreenYellowShift => {
                let t = ((s.t * 0.25).sin() * 0.5 + 0.5);
                let a = Rgbw::MINT;
                let b = Rgbw::YELLOW;
                Rgbw(t.lerp(a.0..b.0), t.lerp(a.1..b.1), t.lerp(a.2..b.2), t.lerp(a.3..b.3))
            }
            BlueGreenShift => {
                let t = ((s.t * 0.25).sin() * 0.5 + 0.5);
                let a = Rgbw::BLUE;
                let b = Rgbw::LIME;
                Rgbw(t.lerp(a.0..b.0), t.lerp(a.1..b.1), t.lerp(a.2..b.2), t.lerp(a.3..b.3))
            }

            Rainbow => {
                let hue = (s.t * 0.05).fract(); // Wraps at 1.0
                let rgb: Rgb = Rgb::hsv(hue, 1.0, 1.0);
                rgb.into()
            }
        }
    }
}
