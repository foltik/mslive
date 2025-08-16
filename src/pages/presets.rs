#![allow(unused)]

use std::cell::RefCell;
use std::collections::HashSet;

use egui::Key;
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
use crate::utils::Swatch;

///////////////////////// PRESETS /////////////////////////

palette! {
    Key::Num1; (0, 7) => Off,
    Key::Num2; (1, 7) => House,
    Key::Num3; (2, 7) => Dimmers,
    Key::Num4; (0, 5) => Sine,
    Key::Num5; (7, 7) => Random,
    Key::Num6; (2, 5) => BarSweep::default(),
    Key::Num7; (3, 5) => BarDirect::default(),
    Key::Num8; (4, 5) => RandomChase::default(),
    Key::Num9; (5, 5) => Chauvet::default(),
}

struct Random;
impl Preset for Random {
    fn color(&self, _t: f64) -> Rgb {
        // color of the button on the launchpad
        Rgb::WHITE
    }

    fn lights(&mut self, t: f64, l: &mut Lights) {
        // t is a float representing the number of seconds elapsed since program start
        // the fract() returns the decimal part, so 123.456 would return 0.456
        let time_fract = t.fract();

        // this color alternates between WHITE and BLACK every 0.5s
        let color = if time_fract > 0.5 { Rgbw::WHITE } else { Rgbw::BLACK };

        // l.rgbw: an array of colors that gets mapped to each RGB bar.
        // for example: l.rgbw[0] is an Rgbw type: Rgbw(0.5, 1.0, 0.25, 0.3),
        // where each number is the red, green, blue, white channel respectively
        for rgbw_color in &mut l.rgbw {
            *rgbw_color = color;
        }

        // l.dimmer: an array of brightnesses from 0.0 to 1.0 for each dimmer pack
        // for example: l.dimmer[0] is a float
        for dimmer_brightness in &mut l.dimmer {
            *dimmer_brightness = 1.0;
        }

        // to see all the available properties on `l`, go to `lights.rs`
    }
}

struct Off;
impl Preset for Off {
    fn color(&self, _t: f64) -> Rgb {
        Rgb::BLACK
    }
    fn lights(&mut self, _t: f64, _l: &mut Lights) {}
}

struct House;
impl Preset for House {
    fn color(&self, _t: f64) -> Rgb {
        // Rgb::HOUSE
        Rgb::hsv((_t / 4.0).fract(), 1.0, 1.0)
    }

    fn lights(&mut self, _t: f64, l: &mut Lights) {
        for rgbw in &mut l.rgbw {
            *rgbw = Rgb::hsv((_t / 4.0).fract(), 1.0, 1.0).into();
        }
        // for dimmer in &mut l.dimmer {
        //     *dimmer = 1.0;
        // }
    }
}

struct Dimmers;
impl Preset for Dimmers {
    fn color(&self, _t: f64) -> Rgb {
        Rgb::WHITE
    }

    fn lights(&mut self, _t: f64, l: &mut Lights) {
        for dimmer in &mut l.dimmer {
            *dimmer = 1.0;
        }
    }
}

struct Sine;
impl Preset for Sine {
    fn color(&self, t: f64) -> Rgb {
        Rgb::WHITE * t.fsin(3.0)
    }

    fn lights(&mut self, t: f64, l: &mut Lights) {
        let fr = t.fsin(3.0);
        for rgbw in &mut l.rgbw {
            *rgbw = Rgbw::HOUSE * fr;
        }
    }
}

struct BarSweep {
    scanner: [f64; 8],
    angle_speed: f64,
    chase_speed: f64,
    lfo_freq: f64,
    lfo_zero_hold: f64,
    hue: f64,
    angle_phase: f64,
    chase_phase: f64,
    lfo_phase: f64,
    last_t: f64,
}

impl Default for BarSweep {
    fn default() -> Self {
        Self {
            scanner: [0.0; 8],
            angle_speed: 0.0,
            chase_speed: 0.0,
            lfo_freq: 0.0,
            lfo_zero_hold: 0.0,
            hue: 0.0,
            angle_phase: 0.0,
            chase_phase: 0.0,
            lfo_phase: 0.0,
            last_t: 0.0,
        }
    }
}

impl Preset for BarSweep {
    fn color(&self, _t: f64) -> Rgb {
        Rgb::WHITE
    }

    fn lights(&mut self, t: f64, l: &mut Lights) {
        let dt = t - self.last_t;
        self.last_t = t;

        // Update phases with delta time to avoid jumps
        self.angle_phase += dt * self.angle_speed * 6.28318;
        self.chase_phase += dt * self.chase_speed * 6.28318;

        // LFO phase runs slower when there's hold time
        let lfo_speed_factor = 1.0 / (1.0 + self.lfo_zero_hold);
        self.lfo_phase += dt * self.lfo_freq * 6.28318 * lfo_speed_factor;

        // Wrap phases to avoid overflow
        self.angle_phase = self.angle_phase % 6.28318;
        self.chase_phase = self.chase_phase % 6.28318;
        self.lfo_phase = self.lfo_phase % 6.28318;

        // Bar angle from phase
        l.bar.angle = (self.angle_phase / 6.28318).fract();

        // Single pixel scanning from phase
        let pixel_pos = ((self.chase_phase / 6.28318).fract() * 10.0) as usize;

        // LFO brightness modulation with zero hold extension
        let lfo_brightness = if self.lfo_freq > 0.0 {
            // Get normalized phase (0..1) from accumulated phase
            let normalized_phase = (self.lfo_phase / 6.28318) % 1.0;

            // Split cycle: sine portion vs hold portion
            let sine_portion = 1.0 / (1.0 + self.lfo_zero_hold);

            if normalized_phase < sine_portion {
                // Scale phase to fit in sine portion and create normal sine wave
                let scaled_phase = (normalized_phase / sine_portion) * 6.28318;
                scaled_phase.sin() * 0.5 + 0.5
            } else {
                // Hold at zero
                0.0
            }
        } else {
            1.0
        };

        // HSV color with controlled hue (0.0..1.0 range)
        let pixel_color = Rgbw::from(Rgb::hsv(self.hue, 1.0, lfo_brightness));

        for (i, bead) in l.bar.beads.iter_mut().enumerate() {
            if i == pixel_pos {
                *bead = pixel_color;
            } else {
                *bead = Rgbw::BLACK;
            }
        }

        for i in 0..7 {
            l.scanner1.raw[i] = self.scanner[i].clamp(0.0, 1.0).lerp(0..255) as u8;
        }
    }

    fn on_ctrl(&mut self, event: launch_control_xl::Input) {
        use launch_control_xl::*;
        match event {
            Input::SendB(0, fr) => self.angle_speed = (fr + 1.0) / 2.0 * 0.2, // Convert -1..1 to 0..0.2 for angle rotation speed
            Input::SendB(1, fr) => self.chase_speed = (fr + 1.0) / 2.0 * 2.0, // Convert -1..1 to 0..2 for chase speed range
            Input::SendB(2, fr) => self.lfo_freq = (fr + 1.0) / 2.0 * 5.0,    // Convert -1..1 to 0..5 Hz for LFO frequency
            Input::SendB(3, fr) => self.lfo_zero_hold = (fr + 1.0) / 2.0,     // Convert -1..1 to 0..1 for zero hold time ratio
            Input::SendB(4, fr) => self.hue = (fr + 1.0) / 2.0,               // Convert -1..1 to 0..1 for hue
            Input::Slider(i, fr) => self.scanner[i as usize] = fr,
            _ => {}
        }
    }
}

struct BarDirect {
    angle: f64,
    target_angle: f64,
    chase_speed: f64,
    lfo_freq: f64,
    lfo_zero_hold: f64,
    hue: f64,
    chase_phase: f64,
    lfo_phase: f64,
    last_t: f64,
}

impl Default for BarDirect {
    fn default() -> Self {
        Self {
            angle: 0.0,
            target_angle: 0.0,
            chase_speed: 0.0,
            lfo_freq: 0.0,
            lfo_zero_hold: 0.0,
            hue: 0.0,
            chase_phase: 0.0,
            lfo_phase: 0.0,
            last_t: 0.0,
        }
    }
}

impl Preset for BarDirect {
    fn color(&self, _t: f64) -> Rgb {
        Rgb::WHITE
    }

    fn lights(&mut self, t: f64, l: &mut Lights) {
        let dt = t - self.last_t;
        self.last_t = t;

        // Update phases with delta time to avoid jumps
        self.chase_phase += dt * self.chase_speed * 6.28318;
        self.lfo_phase += dt * self.lfo_freq * 6.28318;

        // Wrap phases to avoid overflow
        self.chase_phase = self.chase_phase % 6.28318;
        self.lfo_phase = self.lfo_phase % 6.28318;

        // Smooth angle interpolation with wrapping
        let angle_diff = self.target_angle - self.angle;
        let wrapped_diff = if angle_diff > 0.5 {
            angle_diff - 1.0
        } else if angle_diff < -0.5 {
            angle_diff + 1.0
        } else {
            angle_diff
        };

        // Interpolate towards target (adjust speed as needed)
        let interp_speed = 5.0; // Higher = faster interpolation
        self.angle += wrapped_diff * dt * interp_speed;
        self.angle = self.angle.fract(); // Keep in 0..1 range
        if self.angle < 0.0 {
            self.angle += 1.0;
        }

        // Bar angle from interpolated value
        l.bar.angle = self.angle;

        // Single pixel scanning from phase
        let pixel_pos = ((self.chase_phase / 6.28318).fract() * 10.0) as usize;

        // LFO brightness modulation with zero hold extension
        let lfo_brightness = if self.lfo_freq > 0.0 {
            // Get normalized phase (0..1) from accumulated phase
            let normalized_phase = (self.lfo_phase / 6.28318) % 1.0;

            // Split cycle: sine portion vs hold portion
            let sine_portion = 1.0 / (1.0 + self.lfo_zero_hold);

            if normalized_phase < sine_portion {
                // Scale phase to fit in sine portion and create normal sine wave
                let scaled_phase = (normalized_phase / sine_portion) * 6.28318;
                scaled_phase.sin() * 0.5 + 0.5
            } else {
                // Hold at zero
                0.0
            }
        } else {
            1.0
        };

        // HSV color with controlled hue (0.0..1.0 range)
        let pixel_color = Rgbw::from(Rgb::hsv(self.hue, 1.0, lfo_brightness));

        for (i, bead) in l.bar.beads.iter_mut().enumerate() {
            if i == pixel_pos {
                *bead = pixel_color;
            } else {
                *bead = Rgbw::BLACK;
            }
        }
    }

    fn on_ctrl(&mut self, event: launch_control_xl::Input) {
        use launch_control_xl::*;
        match event {
            Input::SendB(0, fr) => self.target_angle = (fr + 1.0) / 2.0, // Convert -1..1 to 0..1 for target angle
            Input::SendB(1, fr) => self.chase_speed = (fr + 1.0) / 2.0 * 2.0, // Convert -1..1 to 0..2 for chase speed range
            Input::SendB(2, fr) => self.lfo_freq = (fr + 1.0) / 2.0 * 5.0, // Convert -1..1 to 0..5 Hz for LFO frequency
            Input::SendB(3, fr) => self.lfo_zero_hold = (fr + 1.0) / 2.0, // Convert -1..1 to 0..1 for zero hold time ratio
            Input::SendB(4, fr) => self.hue = (fr + 1.0) / 2.0,          // Convert -1..1 to 0..1 for hue
            _ => {}
        }
    }
}

struct RandomChase {
    angle_speed: f64,
    random_speed: f64,
    hue: f64,
    sequence: Vec<usize>,
    current_index: usize,
    last_step_time: f64,
    step_interval: f64,
    angle_phase: f64,
    last_t: f64,
}

impl Default for RandomChase {
    fn default() -> Self {
        use std::collections::HashSet;

        // Generate initial random sequence of 0..9 without repeats
        let mut sequence = Vec::new();
        let mut remaining: HashSet<usize> = (0..10).collect();

        // Simple pseudo-random sequence generation (deterministic but chaotic)
        let mut seed = 42u64;
        while !remaining.is_empty() {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let index = (seed as usize) % remaining.len();
            let value = *remaining.iter().nth(index).unwrap();
            remaining.remove(&value);
            sequence.push(value);
        }

        Self {
            angle_speed: 0.0,
            random_speed: 0.5,
            hue: 0.0,
            sequence,
            current_index: 0,
            last_step_time: 0.0,
            step_interval: 1.0,
            angle_phase: 0.0,
            last_t: 0.0,
        }
    }
}

impl Preset for RandomChase {
    fn color(&self, _t: f64) -> Rgb {
        Rgb::WHITE
    }

    fn lights(&mut self, t: f64, l: &mut Lights) {
        let dt = t - self.last_t;
        self.last_t = t;

        // Update angle phase for smooth rotation
        self.angle_phase += dt * self.angle_speed * 6.28318;
        self.angle_phase = self.angle_phase % 6.28318;

        // Triangle wave: 0→1→0 (no wrapping jump)
        let normalized_phase = self.angle_phase / 6.28318;
        l.bar.angle = if normalized_phase < 0.5 {
            normalized_phase * 2.0 // 0→1 in first half
        } else {
            2.0 - normalized_phase * 2.0 // 1→0 in second half
        };

        // Update step timing for random sequence
        if t - self.last_step_time >= self.step_interval {
            self.last_step_time = t;
            self.current_index = (self.current_index + 1) % self.sequence.len();

            // Generate new random sequence when we complete a cycle
            if self.current_index == 0 {
                self.generate_new_sequence(t);
            }
        }

        // Set step interval based on random speed (faster speed = shorter interval)
        self.step_interval = if self.random_speed > 0.0 { 1.0 / self.random_speed } else { 10.0 };

        // Light up current bead in sequence
        let current_bead = self.sequence[self.current_index];
        let pixel_color = Rgbw::from(Rgb::hsv(self.hue, 1.0, 1.0));

        for (i, bead) in l.bar.beads.iter_mut().enumerate() {
            if i == current_bead {
                *bead = pixel_color;
            } else {
                *bead = Rgbw::BLACK;
            }
        }
    }

    fn on_ctrl(&mut self, event: launch_control_xl::Input) {
        use launch_control_xl::*;
        match event {
            Input::SendB(0, fr) => self.angle_speed = (fr + 1.0) / 2.0 * 0.6, // Convert -1..1 to 0..0.6 for angle rotation speed (3x faster)
            Input::SendB(1, fr) => self.random_speed = (fr + 1.0) / 2.0 * 15.0, // Convert -1..1 to 0..15 for steps per second
            Input::SendB(2, fr) => self.hue = (fr + 1.0) / 2.0,               // Convert -1..1 to 0..1 for hue
            _ => {}
        }
    }
}

impl RandomChase {
    fn generate_new_sequence(&mut self, t: f64) {
        use std::collections::HashSet;

        // Use time as additional entropy for the seed
        let time_seed = (t * 1000.0) as u64;
        let mut seed = time_seed.wrapping_mul(1103515245).wrapping_add(12345);

        let mut new_sequence = Vec::new();
        let mut remaining: HashSet<usize> = (0..10).collect();

        while !remaining.is_empty() {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let index = (seed as usize) % remaining.len();
            let value = *remaining.iter().nth(index).unwrap();
            remaining.remove(&value);
            new_sequence.push(value);
        }

        self.sequence = new_sequence;
    }
}

#[derive(Default)]
struct Chauvet {
    channels: [u8; 8],
}
impl Preset for Chauvet {
    fn color(&self, _t: f64) -> Rgb {
        Rgb::WHITE
    }

    fn lights(&mut self, _t: f64, l: &mut Lights) {
        for i in 0..7 {
            l.scanner1.raw[i] = self.channels[i];
        }
    }

    fn on_ctrl(&mut self, event: launch_control_xl::Input) {
        use launch_control_xl::*;
        match event {
            Input::Slider(i, fr) => self.channels[i as usize] = fr.clamp(0.0, 1.0).lerp(0..255) as u8,
            _ => {}
        }
    }
}

///////////////////////// PAGE /////////////////////////

trait Preset {
    fn color(&self, t: f64) -> Rgb;
    fn lights(&mut self, t: f64, l: &mut Lights);
    fn on_ctrl(&mut self, _event: launch_control_xl::Input) {}
}

pub struct Presets {
    presets: Vec<(Key, Swatch<RefCell<Box<dyn Preset>>>)>,
    preset: usize,
    time: f64,
}

impl Default for Presets {
    fn default() -> Self {
        Self { presets: presets(), preset: 0, time: 0.0 }
    }
}

impl Page for Presets {
    fn tick(&mut self, dt: f64) {
        self.time += dt;
    }

    fn input_ctrl(&mut self, event: launch_control_xl::Input) {
        self.presets[self.preset].1.op.borrow_mut().on_ctrl(event);
    }
    fn input_pad(&mut self, _pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
        let Some((x, y)) = event.xy() else { return };

        if let Some((i, _swatch)) = self.presets.iter().enumerate().find(|(_i, (_k, s))| s.xy == (x, y)) {
            self.preset = i;
        }
    }
    fn input_keys(&mut self, keys: &HashSet<Key>) {
        for (i, (key, _)) in self.presets.iter().enumerate() {
            if keys.contains(key) {
                self.preset = i;
            }
        }
        // if keys.contains(&Key::Num1) {
        // }
    }

    fn output_lights(&self, lights: &mut Lights) {
        self.presets[self.preset].1.op.borrow_mut().lights(self.time, lights);
    }
    fn output_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};

        let mut batch: Vec<(Pos, Color)> = Vec::with_capacity(self.presets.len());
        for (_key, Swatch { xy: (x, y), op }) in self.presets.iter() {
            let Rgb(r, g, b) = op.borrow().color(self.time);
            batch.push((Coord(*x, *y).into(), Color::Rgb(r, g, b)));
        }
        pad.send(Output::Batch(batch));
    }
    fn output_ctrl(&self, _ctrl: &mut Midi<launch_control_xl::LaunchControlXL>) {}
}

macro_rules! palette {
    ($($key:expr ; ($x:expr, $y:expr) => $preset:expr),* $(,)?) => {
        fn presets() -> Vec<(Key, Swatch<RefCell<Box<dyn Preset>>>)> {
            vec![
                $( ($key, Swatch { xy: ($x, $y), op: RefCell::new(Box::new($preset)) }) ),*
            ]
        }
    }
}
use palette;
