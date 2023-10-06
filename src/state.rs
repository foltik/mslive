use std::collections::VecDeque;

use stagebridge::color::Rgbw;
use stagebridge::dmx::Device;
use stagebridge::dmx::device::beam_rgbw_60w::Beam;
use stagebridge::dmx::device::laser_scan_30w::Laser;
use stagebridge::dmx::device::spider_rgbw_8x10w::Spider;
use stagebridge::dmx::device::strobe_rgb_35w::Strobe;
use stagebridge::dmx::device::par_rgbw_12x3w::Par;
use stagebridge::dmx::device::bar_rgb_18w::Bar;
use stagebridge::e131::E131;
use stagebridge::num::Float;

use crate::fx::{self, ColorOp, ColorMapOp};
use crate::color::*;

// different lighting scenes
pub enum Scene {
    Init,

    Buildup,

}

#[derive(Clone)]
pub enum ColorMode {
    Red,
    Green,
    Blue,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub enum ColorSelect {
    All,
    Color0,
    Color1,
}

#[derive(Clone, Default)]
pub struct Lights {
    pub pars: [Par; 10],
    pub beams: [Beam; 4],
    pub strobe: Strobe,
    pub bars: [Bar; 2],
    pub spiders: [Spider; 2],
    pub laser: Laser,
}

impl Lights {
    pub fn beam(&mut self, f: impl Fn(&mut Beam)) {
        for beam in &mut self.beams {
            f(beam);
        }
    }

    pub fn color(&mut self, color: Rgbw) {
        for par in &mut self.pars {
            par.color = color;
        }
        for beam in &mut self.beams {
            beam.color = color;
        }
        for bar in &mut self.bars {
            bar.color = color.into();
        }
        self.strobe.color = color.into();
        for spider in &mut self.spiders {
            spider.color0 = color;
            spider.color1 = color;
        }
    }

    pub fn alpha(&mut self, fr: f64) {
        for par in &mut self.pars {
            par.alpha = fr;
        }
        for beam in &mut self.beams {
            beam.alpha = fr;
        }
        for bar in &mut self.bars {
            bar.alpha = fr;
        }
        self.strobe.alpha = fr;
        for spider in &mut self.spiders {
            spider.alpha = fr;
        }
    }

}

impl State {
    pub fn phi0(&self, pd: Pd) -> f64 {
        self.phi0.mod_div(pd.fr() * self.phi_mod)
    }

    pub fn phi(&self, pd: Pd) -> f64 {
        self.phi.mod_div(pd.fr() * self.phi_mod)
    }

    pub fn dt(&self, pd: Pd) -> f64 {
        self.dt / ((self.bpm / 60.0) * pd.fr())
    }

    pub fn color0(&self) -> Color {
        self.map0.apply(self, self.c0.apply(self))
    }
    pub fn color1(&self) -> Color {
        self.map1.apply(self, self.c1.apply(self))
    }

    pub fn color0_phase(&self, pd: Pd, offset: f64) -> Color {
        let mut state = self.clone();
        state.phi0 = state.phi0.phase(pd.fr(), offset);
        self.map0.apply(&state, self.c0.apply(&state))
    }
    pub fn color1_phase(&self, pd: Pd, offset: f64) -> Color {
        let mut state = self.clone();
        state.phi0 = state.phi0.phase(pd.fr(), offset);
        self.map1.apply(&state, self.c1.apply(&state))
    }
}


impl Lights {
    pub fn send(&self, e131: &mut E131) {
        let mut dmx = [0u8; 205];

        for (i, par) in self.pars.iter().enumerate() {
            par.encode(&mut dmx[1 + 8*i..]);
        }

        for (i, beam) in self.beams.iter().enumerate() {
            beam.encode(&mut dmx[81 + 15 * i..]);
        }

        self.strobe.encode(&mut dmx[142..]);

        for (i, bar) in self.bars.iter().enumerate() {
            bar.encode(&mut dmx[149 + 7 * i..]);
        }

        self.laser.encode(&mut dmx[164..]);

        for (i, spider) in self.spiders.iter().enumerate() {
            spider.encode(&mut dmx[175 + 15*i..]);
        }

        e131.send(&dmx);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pd(pub usize, pub usize);
impl Pd {
    pub fn fr(&self) -> f64 {
        self.0 as f64 / self.1 as f64
    }
    pub fn mul(&self, mul: usize) -> Self {
        Self(self.0 * mul, self.1)
    }
    pub fn div(&self, div: usize) -> Self {
        Self(self.0, self.1 * div)
    }
}
