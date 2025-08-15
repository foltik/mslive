use rand::{seq::SliceRandom, Rng};
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

const CRYSTAL: Rgb = Rgb(0.6, 0.1, 0.1);

#[derive(Default, Debug)]
pub struct Random {
    time: f64,
    last_randomize: f64,

    crystal0: Rgb,
    crystal1: f64,

    scanner1_on: bool,
    scanner1_spread: Env,
    scanner1_scroll: Env,
    scanner1_rotate: Env,
    scanner1_gobo: f64,
    dimmers: DimmersEnv,
    dimmer_order: [usize; 9],

    bar_color: BarColor,
    bar_env: BarEnv,
}

#[derive(Default, Debug)]
pub enum Env {
    #[default]
    Off,
    Const(f64),
    Bounce(f64),
    Alternate(f64, f64),
}

impl Env {
    fn fr(&self, s: &Random) -> f64 {
        match self {
            Env::Off => 0.0,
            Env::Const(fr) => *fr,
            Env::Bounce(fr) => (s.time / 4.0).fsin(1.0) * fr,
            Env::Alternate(fr0, fr1) => match (s.time / 4.0).bsquare(1.0, 0.5) {
                true => *fr0,
                false => *fr1,
            },
        }
    }
}

#[derive(Default, Debug)]
pub enum DimmersEnv {
    #[default]
    Off,
    Chase,
    Sparkle,
    Wave,
}

#[derive(Default, Debug)]
pub enum BarColor {
    #[default]
    Off,
    Red,
    Green,
    Blue,
    RedBlue,
    GreenBlue,
    White,
}

#[derive(Default, Debug)]
pub enum BarEnv {
    #[default]
    Solid,
    #[allow(unused)]
    Chase,
}

impl Random {
    fn reset(&mut self) {
        self.scanner1_on = false;
        self.crystal0 = Rgb::BLACK;
        self.crystal1 = 0.0;
        self.bar_color = BarColor::Off;
        self.dimmers = DimmersEnv::Off;
    }

    pub fn randomize(&mut self) {
        self.last_randomize = self.time;

        let rand_bool = |p| rand::thread_rng().gen_bool(p);

        if rand_bool(0.5) {
            println!("0.5: scanners");
            self.randomize_scanners();
        } else {
            if rand_bool(0.6) {
                println!("0.3: dimmers");
                self.randomize_dimmers();
            } else {
                println!("0.2: other");
                self.randomize_other();
            }
        }
    }

    fn randomize_scanners(&mut self) {
        let rand_f64 = |min: f64, max: f64| rand::thread_rng().gen_range(min..=max);
        let rand_u8 = |max: u8| rand::thread_rng().gen::<u8>() % max;
        let rand_env = || match rand_u8(4) {
            0 => Env::Off,
            1 => Env::Const(rand_f64(0.25, 0.75)),
            2 => Env::Bounce(1.0),
            _ => Env::Alternate(rand_f64(0.0, 0.5), rand_f64(0.5, 1.0)),
        };

        self.reset();
        self.scanner1_on = true;
        self.scanner1_gobo = rand_f64(0.0, 1.0);
        self.scanner1_scroll = rand_env();
        self.scanner1_rotate = rand_env();
        self.scanner1_spread = rand_env();
    }

    fn randomize_dimmers(&mut self) {
        let rand = || rand::thread_rng().gen::<u8>();

        let rand_dimmers_env = || match rand() % 3 {
            0 => DimmersEnv::Chase,
            1 => DimmersEnv::Wave,
            _ => DimmersEnv::Sparkle,
        };
        let rand_dimmer_order = || {
            let mut idxs = (0..9).collect::<Vec<usize>>();
            idxs.shuffle(&mut rand::thread_rng());
            idxs
        };

        self.reset();
        self.dimmers = rand_dimmers_env();
        self.dimmer_order.copy_from_slice(&rand_dimmer_order());
    }

    fn randomize_other(&mut self) {
        let rand_bool = |p| rand::thread_rng().gen_bool(p);
        let rand = || rand::thread_rng().gen::<u8>();

        let rand_bar_color = || match rand() % 6 {
            0 => BarColor::Red,
            1 => BarColor::Green,
            2 => BarColor::Blue,
            3 => BarColor::White,
            4 => BarColor::RedBlue,
            _ => BarColor::GreenBlue,
        };

        self.reset();
        if rand_bool(0.75) {
            println!("0.15: bar + crystals");
            if rand_bool(0.5) {
                println!("0.075: red bar + both crystals");
                self.bar_color = BarColor::Red;
                self.crystal0 = CRYSTAL;
                self.crystal1 = 1.0;
            } else {
                println!("0.075: bar + color crystal");
                self.bar_color = rand_bar_color();
                self.crystal0 = self.bar_color.rgbw().into();
            }
        } else {
            if rand_bool(0.5) {
                println!("0.075: bar only");
                self.bar_color = rand_bar_color();
            } else {
                println!("0.075: crystals only");
                self.crystal0 = CRYSTAL;
                self.crystal1 = 1.0;
            }
        }
    }
}

impl BarColor {
    fn rgbw(&self) -> Rgbw {
        match self {
            BarColor::Off => Rgbw::BLACK,
            BarColor::Red => Rgbw::RED,
            BarColor::Green => Rgbw::LIME,
            BarColor::Blue => Rgbw::BLUE,
            BarColor::RedBlue => Rgbw(1.0, 0.0, 1.0, 0.0),
            BarColor::GreenBlue => Rgbw(0.0, 1.0, 1.0, 0.0),
            BarColor::White => Rgbw(0.0, 0.0, 0.0, 1.0),
        }
    }
}

impl Page for Random {
    fn tick(&mut self, dt: f64) {
        self.time += dt;
        if self.time - self.last_randomize > 60.0 {
            self.randomize();
        }
    }

    fn input_pad(&mut self, _pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
        match event.xy() {
            Some((0, 7)) => self.randomize_scanners(),
            Some((1, 7)) => self.randomize_dimmers(),
            Some((2, 7)) => self.randomize_other(),
            Some(_) => self.randomize(),
            None => {}
        }
    }

    fn input_ctrl(&mut self, _event: launch_control_xl::Input) {}

    fn output_lights(&self, lights: &mut Lights) {
        lights.reset();

        lights.scanner1.on = self.scanner1_on;
        lights.scanner1.gobo = self.scanner1_gobo;
        lights.scanner1.scroll = self.scanner1_scroll.fr(self) * 0.5;
        lights.scanner1.rotate = self.scanner1_rotate.fr(self);
        lights.scanner1.spread = self.scanner1_spread.fr(self);

        let bar_color = self.bar_color.rgbw();
        for (i, bead) in lights.bar.beads.iter_mut().enumerate() {
            let i = i as f64;
            let fr = match self.bar_env {
                BarEnv::Solid => 1.0,
                BarEnv::Chase => self.time.fract().phase(1.0, 0.1 * i).square(1.0, 0.5),
            };
            *bead = bar_color * fr * 0.05;
        }
        lights.bar.angle = match self.bar_color {
            BarColor::Off => 0.5,
            _ => (self.time / 20.0).fsin(1.0) * 0.5 + 0.25,
        };

        match self.dimmers {
            DimmersEnv::Off => {}
            DimmersEnv::Chase => {
                let total = self.dimmer_order.len() as f64;
                let head = (self.time / 1.5).fmod(total);

                for (n, &i) in self.dimmer_order.iter().enumerate() {
                    let dist = (n as f64 - head + total).fmod(total);
                    lights.dimmer[i] = (1.0 - dist / 3.0).powf(1.5);
                }
            }
            DimmersEnv::Sparkle => {
                let total = self.dimmer_order.len() as f64;
                let pos = (self.time / 2.0).tri(1.0) * (total - 1.0);

                for (n, &i) in self.dimmer_order.iter().enumerate() {
                    let dist = (n as f64 - pos).abs();
                    lights.dimmer[i] = (1.0 - dist).clip(0.0..1.0).powf(2.0);
                }
            }
            DimmersEnv::Wave => {
                let wave_period = 10.0;
                let wave_width = 6.0;
                let fade_exponent = 2.0;

                let t = self.time.fmod(wave_period);

                for (n, &i) in self.dimmer_order.iter().enumerate() {
                    let phase = (t + n as f64 * 0.4).fmod(wave_period);
                    if phase < wave_width {
                        let fr = (1.0 - (phase / wave_width)).clip(0.0..1.0);
                        lights.dimmer[i] = fr.powf(fade_exponent);
                    } else {
                        lights.dimmer[i] = 0.0;
                    }
                }
            }
        }

        for dimmer in &mut lights.dimmer {
            *dimmer = *dimmer;
        }

        lights.crystal0 = self.crystal0;
        lights.crystal1 = self.crystal1;
    }

    fn output_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};
        pad.send(Output::Rgb(Coord(0, 7).into(), Rgb(1.0, 1.0, 1.0)));
        pad.send(Output::Rgb(Coord(1, 7).into(), Rgb(1.0, 1.0, 1.0)));
        pad.send(Output::Rgb(Coord(2, 7).into(), Rgb(1.0, 1.0, 1.0)));
    }

    fn output_ctrl(&self, _ctrl: &mut Midi<launch_control_xl::LaunchControlXL>) {}
}
