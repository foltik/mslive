use std::sync::{Arc, Mutex};

use stagebridge::num::{Float, Ease, Range};

use crate::state::{State, Pd};
use crate::color::Color;

macro_rules! op {
    ($trait:ident, $boxed:ident, ($($arg:ident : $ty:ty),*) -> $ret:ty) => {
        pub trait $trait: Send + Sync + 'static {
            fn box_clone(&self) -> Box<dyn $trait>;
            fn apply(&mut self, $($arg: $ty),*) -> $ret;
        }

        impl<F: FnMut($($ty),*) -> $ret + Send + Sync + Clone + 'static > $trait for F {
            fn box_clone(&self) -> Box<dyn $trait> {
                Box::new(self.clone())
            }
            fn apply(&mut self, $($arg: $ty),*) -> $ret {
                (self)($($arg),*)
            }
        }
        pub struct $boxed(Arc<Mutex<Box<dyn $trait>>>);
        impl $boxed {
            pub fn new<F: $trait>(func: F) -> Self {
                Self(Arc::new(Mutex::new(Box::new(func))))
            }
            pub fn value(value: $ret) -> Self {
                Self::new(move |$(_: $ty),*| value)
            }
            pub fn apply(&self, $($arg : $ty),*) -> $ret {
                self.0.lock().unwrap().apply($($arg),*)
            }
        }
        impl<F: FnMut($($ty),*) -> $ret + Send + Sync + Clone + 'static> From<F> for $boxed {
            fn from(func: F) -> Self {
                Self::new(func)
            }
        }

        impl Clone for $boxed {
            fn clone(&self) -> Self {
                Self(Arc::clone(&self.0))
            }
        }
    };
    ($trait:ident, $boxed:ident, ($($arg:ident : $ty:ty),*)) => {
        op!($trait, $boxed, ($($arg : $ty),*) -> ());
    };
}

op!(ColorFn, ColorOp, (state: &State) -> Color);
impl From<Color> for ColorOp {
    fn from(color: Color) -> Self {
        ColorOp::value(color)
    }
}

op!(ColorMapFn, ColorMapOp, (state: &State, color: Color) -> Color);
impl ColorMapOp {
    pub fn compose(self, other: ColorMapOp) -> ColorMapOp {
        ColorMapOp::new(move |state: &State, color: Color| other.apply(state, self.apply(state, color)))
    }
}
impl From<Color> for ColorMapOp {
    fn from(color: Color) -> Self {
        ColorMapOp::value(color)
    }
}

// op!(LightFn, LightOp, (state: &State, lights: &mut Lights));

// op!(LightColorFn, LightColorOp, (color: Color, lights: &mut Lights));

pub fn rainbow(pd: Pd) -> ColorOp {
    ColorOp::new(move |state: &State| Color::hsv(state.phi(pd), 1.0, 1.0))
}

pub fn sin(pd: Pd, a: f64, range: f64) -> ColorMapOp {
    ColorMapOp::new(move |state: &State, color: Color| {
        let t = state.phi(pd);
        color.a(a + t.ssin(1.0) * range)
    })
}

pub fn pulse<R: Into<Range>>(pd: Pd, range: R) -> ColorMapOp {
    let range = range.into();
    ColorMapOp::new(move |state: &State, color: Color| {
        let fr = state.phi(pd).ease_quad_out();
        color.a(fr.lerp(range))
    })
}

pub fn pulse_short<R: Into<Range>>(pd: Pd, range: R) -> ColorMapOp {
    let range = range.into();
    ColorMapOp::new(move |state: &State, color: Color| {
        let fr = state.phi(pd).ease_cubic_out();
        color.a(fr.lerp(range))
    })
}

pub fn ramp(pd: Pd) -> ColorMapOp {
    ColorMapOp::new(move |state: &State, color: Color| {
        let fr = 1.0 - state.phi(pd);
        color.a(fr)
    })
}

pub fn tri<R: Into<Range>>(pd: Pd, range: R) -> ColorMapOp {
    let range = range.into();
    ColorMapOp::new(move |state: &State, color: Color| {
        let fr = state.phi(pd).tri(1.0);
        color.a(fr.lerp(range))
    })
}

pub fn strobe<R: Into<Range>>(pd: Pd, duty: f64, range: R) -> ColorMapOp {
    let range = range.into();
    ColorMapOp::new(move |state: &State, color: Color| {
        let fr = state.phi(pd).square(1.0, duty);
        color.a(fr.lerp(range))
    })
}

pub fn once(pd: Pd, op: ColorMapOp) -> ColorMapOp {
    let mut start: Option<f64> = None;
    let mut done = false;
    ColorMapOp::new(move |state: &State, color| {
        let t = match start {
            Some(phi) => {
                let wrap = if state.phi < phi {
                    state.phi + 16.0 - phi
                } else {
                    state.phi - phi
                };
                if done || wrap > pd.fr() - f64::EPSILON {
                    done = true;
                    pd.fr() - f64::EPSILON
                } else {
                    wrap
                }
            }
            None => {
                start = Some(state.phi);
                0.0
            },
        };

        let mut state = state.clone();
        state.phi = t;

        let color = op.apply(&state, color);
        color
    })
}

pub fn off() -> ColorMapOp {
    Color::OFF.into()
}

pub fn id() -> ColorMapOp {
    ColorMapOp::new(|_: &State, color| color)
}


pub fn alpha(fr: f64) -> ColorMapOp {
    ColorMapOp::new(move |_: &State, color: Color| color.a(fr))
}
