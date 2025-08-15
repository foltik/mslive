use stagebridge::color::{Rgb, Rgbw};

use stagebridge::midi::device::{
    launch_control_xl::{self},
    launchpad_x::{self, LaunchpadX},
};
use stagebridge::midi::Midi;
use stagebridge::prelude::*;

use crate::lights::Lights;

///////////////////////// STATE /////////////////////////

pub struct Generator {
    x: i8,

    time: f64,
    rgbw_decay: f64,
    speed: f64,
    offset: f64,
    color: Color,

    rgbw_env: [f64; 9],
    rgbw_alpha: f64,
    rgbw_range: f64,

    dimmer_env: [f64; 8],
    dimmer_alpha: f64,
    dimmer_decay: f64,
}

#[derive(Clone, Copy)]
enum Color {
    Solid(Rgbw),
    RedShift,
    GreenShift,
    BlueShift,
    Rainbow,
}

#[derive(Clone, Copy)]
pub struct Swatch<T> {
    pub xy: (i8, i8),
    pub op: T,
}

#[derive(Clone, Copy)]
enum Op {
    Color(Color),
    RgbwEnv(usize),
    RgbwEnvAll,
    DimmerEnv(usize),
}

#[rustfmt::skip]
const PALETTE: &[Swatch<Op>] = &[
    /* ───────── colors ───────── */
    Swatch { xy: (0, 7), op: Op::Color(Color::RedShift) },
    Swatch { xy: (1, 7), op: Op::Color(Color::GreenShift) },
    Swatch { xy: (2, 7), op: Op::Color(Color::BlueShift) },
    Swatch { xy: (0, 6), op: Op::Color(Color::Solid(Rgbw::RED)) },
    Swatch { xy: (1, 6), op: Op::Color(Color::Solid(Rgbw::ORANGE)) },
    Swatch { xy: (2, 6), op: Op::Color(Color::Solid(Rgbw::YELLOW)) },
    Swatch { xy: (0, 5), op: Op::Color(Color::Solid(Rgbw::LIME)) },
    Swatch { xy: (1, 5), op: Op::Color(Color::Solid(Rgbw::MINT)) },
    Swatch { xy: (2, 5), op: Op::Color(Color::Solid(Rgbw::CYAN)) },
    Swatch { xy: (0, 4), op: Op::Color(Color::Solid(Rgbw::BLUE)) },
    Swatch { xy: (1, 4), op: Op::Color(Color::Solid(Rgbw::VIOLET)) },
    Swatch { xy: (2, 4), op: Op::Color(Color::Solid(Rgbw::MAGENTA)) },
    Swatch { xy: (0, 3), op: Op::Color(Color::Solid(Rgbw::WHITE)) },
    Swatch { xy: (1, 3), op: Op::Color(Color::Rainbow) },
    /* ──────── rgbw env ──────── */
    Swatch { xy: (0, 0), op: Op::RgbwEnv(0) },
    Swatch { xy: (1, 0), op: Op::RgbwEnv(1) },
    Swatch { xy: (2, 0), op: Op::RgbwEnv(2) },
    Swatch { xy: (0, 1), op: Op::RgbwEnv(3) },
    Swatch { xy: (1, 1), op: Op::RgbwEnv(4) },
    Swatch { xy: (2, 1), op: Op::RgbwEnv(5) },
    Swatch { xy: (0, 2), op: Op::RgbwEnv(6) },
    Swatch { xy: (1, 2), op: Op::RgbwEnv(7) },
    Swatch { xy: (2, 2), op: Op::RgbwEnv(8) },
    Swatch { xy: (2, 3), op: Op::RgbwEnvAll },
    /* ──────── dimmer env ──────── */
    Swatch { xy: (3, 7), op: Op::DimmerEnv(1) },
    Swatch { xy: (4, 7), op: Op::DimmerEnv(0) },
    Swatch { xy: (3, 6), op: Op::DimmerEnv(3) },
    Swatch { xy: (4, 6), op: Op::DimmerEnv(2) },
    Swatch { xy: (3, 5), op: Op::DimmerEnv(5) },
    Swatch { xy: (4, 5), op: Op::DimmerEnv(4) },
    Swatch { xy: (3, 4), op: Op::DimmerEnv(7) },
    Swatch { xy: (4, 4), op: Op::DimmerEnv(6) },
];

impl Generator {
    pub fn new(x: i8) -> Self {
        Self {
            x,

            time: 0.0,
            speed: 1.0,
            offset: 0.0,
            color: Color::Solid(Rgbw::HOUSE),

            rgbw_env: Default::default(),
            rgbw_alpha: 1.0,
            rgbw_range: 1.0,
            rgbw_decay: 0.5,

            dimmer_env: Default::default(),
            dimmer_alpha: 1.0,
            dimmer_decay: 0.5,
        }
    }

    pub fn tick(&mut self, dt: f64) {
        self.time += dt * self.speed;

        for env in &mut self.rgbw_env {
            *env = (*env - dt * self.rgbw_decay).max(0.0);
        }
        for env in &mut self.dimmer_env {
            *env = (*env - dt * self.dimmer_decay).max(0.0);
        }
    }

    pub fn render(&self, l: &mut Lights) {
        for i in 0..9 {
            l.rgbw[i] += self.render_rgbw(i);
        }
        for i in 0..8 {
            l.dimmer[i] = l.dimmer[i].max(self.render_dimmer(i));
        }
    }
    pub fn render_rgbw(&self, i: usize) -> Rgbw {
        let color = self.color.render(self.time);
        let env = self.rgbw_env[i];
        color * ((env * self.rgbw_alpha * self.rgbw_range) + self.offset)
    }
    pub fn render_dimmer(&self, i: usize) -> f64 {
        self.dimmer_env[i] * self.dimmer_alpha
    }

    pub fn render_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};

        let mut batch: Vec<(Pos, Color)> = Vec::with_capacity(PALETTE.len());
        for Swatch { xy: (x, y), op } in PALETTE.iter().copied() {
            let Rgb(r, g, b) = match op {
                Op::Color(c) => c.render(self.time).into(),
                Op::RgbwEnv(i) => Rgb::WHITE * self.rgbw_env[i],
                Op::RgbwEnvAll => Rgb::BLACK,
                Op::DimmerEnv(i) => Rgb::WHITE * self.dimmer_env[i],
            };
            batch.push((Coord(x + self.x, y).into(), Color::Rgb(r, g, b)));
        }
        pad.send(Output::Batch(batch));
    }

    pub fn handle_pad(&mut self, event: launchpad_x::Input) {
        log::debug!("pad: {event:?}");

        let Some((x, y)) = event.xy() else { return };
        let x = x - self.x;

        if let Some(swatch) = PALETTE.iter().find(|s| s.xy == (x, y)) {
            match swatch.op {
                Op::Color(c) => self.color = c,
                Op::RgbwEnv(i) => self.rgbw_env[i] = 1.0,
                Op::RgbwEnvAll => self.rgbw_env.iter_mut().for_each(|v| *v = 1.0),
                Op::DimmerEnv(i) => self.dimmer_env[i] = 1.0,
            }
        }
    }

    pub fn handle_ctrl(&mut self, event: launch_control_xl::Input) {
        use launch_control_xl::*;
        log::debug!("ctrl: {event:?}");

        match event {
            /* ──────── speed/offset knobs ───────── */
            Input::Pan(i, fr) if i == self.x as u8 + 0 => self.rgbw_range = fr.ilerp(-1.0..1.0),
            Input::Pan(i, fr) if i == self.x as u8 + 1 => self.dimmer_alpha = fr.ilerp(-1.0..1.0),
            Input::Pan(i, fr) if i == self.x as u8 + 2 => self.speed = fr.ilerp(-1.0..1.0) * 4.0,
            Input::Pan(i, fr) if i == self.x as u8 + 3 => self.offset = fr.ilerp(-1.0..1.0),
            /* ───────── envelope sliders ───────── */
            Input::Slider(i, fr) if i == self.x as u8 + 0 => self.rgbw_decay = fr,
            Input::Slider(i, fr) if i == self.x as u8 + 1 => self.rgbw_alpha = fr,
            Input::Slider(i, fr) if i == self.x as u8 + 2 => self.dimmer_decay = fr,
            _ => {}
        }
    }
}

impl Color {
    fn render(self, time: f64) -> Rgbw {
        let shift = |a: Rgbw, b: Rgbw| {
            let t = (time * 0.25).fsin(1.0);
            Rgbw(t.lerp(a.0..b.0), t.lerp(a.1..b.1), t.lerp(a.2..b.2), t.lerp(a.3..b.3))
        };
        match self {
            Color::Solid(rgbw) => rgbw,
            Color::RedShift => shift(Rgbw::RED, Rgbw::VIOLET),
            Color::GreenShift => shift(Rgbw::MINT, Rgbw::YELLOW),
            Color::BlueShift => shift(Rgbw::BLUE, Rgbw::LIME),
            Color::Rainbow => {
                let hue = (time * 0.05).fract();
                Rgb::hsv(hue, 1.0, 1.0).into()
            }
        }
    }
}
