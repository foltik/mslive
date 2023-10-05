use crate::fx::{self, ColorOp, ColorMapOp};
use crate::color::*;

#[derive(Clone)]
pub struct State {
    pub t0: f64,
    pub t: f64,
    pub phi: f64,
    pub phi_mul: f64,
    pub bpm: f64,

    pub viz_pd: Pd,
    pub viz_beat: bool,
    pub viz_beat_last: bool,
    pub viz_alpha: f64,

    pub color_mode: ColorMode,
    pub color0: ColorOp,
    pub color1: ColorOp,
    pub map0: ColorMapOp,
    pub map1: ColorMapOp,

    pub fr0: f64,
    pub fr1: f64,

    pub off: bool,
    pub alpha: f64,
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

impl Default for State {
    fn default() -> Self {
        Self {
            t0: 0.0,
            t: 0.0,
            phi: 0.0,
            phi_mul: 1.0,
            bpm: 120.0,

            viz_pd: Pd(1, 1),
            viz_beat: true,
            viz_beat_last: false,
            viz_alpha: 1.0,

            color_mode: ColorMode::Other,
            color0: ColorOp::value(Color::WHITE),
            color1: ColorOp::value(Color::WHITE),
            map0: fx::id(),
            map1: fx::id(),

            fr0: 0.0,
            fr1: 0.0,

            off: false,
            alpha: 1.0,
        }
    }
}
