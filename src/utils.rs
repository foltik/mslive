use std::ops::Deref;
use std::rc::Rc;

use stagebridge::prelude::*;

use crate::State;

/// Pd
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

impl Default for Pd {
    fn default() -> Self {
        Self(1, 1)
    }
}

// TODO: explain
#[derive(Clone, Copy, Debug, Default)]
pub enum Holdable<T> {
    #[default]
    Off,
    Held {
        x: i8,
        y: i8,
        val: T,
    },
}

impl<T> Holdable<T> {
    pub fn hold(&mut self, x: i8, y: i8, b: bool, val: T) {
        match *self {
            Holdable::Off => *self = Self::Held { x, y, val },
            Holdable::Held { x: x0, y: y0, .. } => match b {
                true => *self = Self::Held { x, y, val },
                false => {
                    if x == x0 && y == y0 {
                        *self = Self::Off
                    }
                }
            },
        }
    }

    // clone to avoid double borrowing state
    pub fn or(&self, fallback: &T) -> T
    where
        T: Clone,
    {
        match self {
            Holdable::Off => fallback.clone(),
            Holdable::Held { val, .. } => val.clone(),
        }
    }
}

/// Beat
#[derive(Default, Clone, Copy)]
pub enum Beat {
    #[default]
    Off,
    On {
        t: f64,
        pd: Pd,
        r: Range,
    },
    Fr(f64),
}

impl Beat {
    pub fn at(s: &State, pd: Pd, r: impl Into<Range>) -> Self {
        Beat::On { t: s.t, pd, r: r.into() }
    }

    pub fn or(&self, s: &State, fallback: f64) -> f64 {
        match *self {
            Beat::Off => fallback,
            Beat::On { t, pd, r, .. } => {
                let dt = s.t - t;
                let len = (60.0 / s.bpm) * pd.fr();

                if dt >= len {
                    r.lo
                } else {
                    (dt / len).ramp(1.0).inv().lerp(r)
                }
            }
            Beat::Fr(fr) => fr,
        }
    }
}

/// Op
pub trait OpFn<T>: Fn(&mut State) -> T + 'static {}
impl<T, F> OpFn<T> for F where F: Fn(&mut State) -> T + 'static {}

pub struct Op<T>(Rc<dyn OpFn<T>>);

impl<T> Op<T> {
    pub fn f(f: impl OpFn<T>) -> Self {
        Self(Rc::new(f))
    }

    pub fn v(t: T) -> Self
    where
        T: Copy + 'static,
    {
        Self::f(move |_| t)
    }
}

impl<T: Default + Copy + 'static> Default for Op<T> {
    fn default() -> Self {
        Self::v(T::default())
    }
}

impl<T> Deref for Op<T> {
    type Target = Rc<dyn OpFn<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for Op<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T, F> From<F> for Op<T>
where
    F: Fn(&mut State) -> T + 'static,
{
    fn from(f: F) -> Self {
        Self::f(f)
    }
}

/// Rgbw
pub trait RgbwExt {
    fn e(self) -> egui::Color32;
}

impl RgbwExt for Rgbw {
    fn e(self) -> egui::Color32 {
        let Rgb(r, g, b) = self.into();
        egui::Color32::from_rgba_premultiplied(r.byte(), g.byte(), b.byte(), 255)
    }
}
