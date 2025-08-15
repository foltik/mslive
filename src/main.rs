use anyhow::Result;
use clap::Parser;
use stagebridge::num::Interp;
use std::time::Instant;

use stagebridge::midi::Midi;
use stagebridge::{
    color::Rgb,
    midi::device::{
        launch_control_xl::{self, LaunchControlXL},
        launchpad_x::{self, LaunchpadX},
    },
};

mod dmx;
mod generator;
mod gui;
mod lights;
mod presets;
mod random;

use generator::Generator;
use lights::Lights;
use presets::Presets;
use random::Random;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Log verbosity. Add more v's for more verbosity.
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,
}

enum Page {
    Generators,
    Presets,
    Random,
}

pub struct State {
    page: Page,
    fx0: Generator,
    fx1: Generator,
    presets: Presets,
    random: Random,

    brightness: f64,
    test0: [f64; 8],

    time: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            page: Page::Generators,
            fx0: Generator::new(0),
            fx1: Generator::new(5),
            presets: Presets::default(),
            random: Random::default(),

            brightness: 1.0,
            test0: [0.0; 8],
            time: 0.0,
        }
    }
}

impl State {
    pub fn tick(&mut self, dt: f64) {
        self.time += dt;
        match self.page {
            Page::Generators => {
                self.fx0.tick(dt);
                self.fx1.tick(dt);
            }
            Page::Presets => self.presets.tick(dt),
            Page::Random => self.random.tick(dt),
        }
    }
    pub fn render(&self, lights: &mut Lights) {
        match self.page {
            Page::Generators => {
                self.fx0.render(lights);
                self.fx1.render(lights);
            }
            Page::Presets => self.presets.render(lights),
            Page::Random => self.random.render(lights),
        }

        // Apply global brightness multiplier
        for rgbw in &mut lights.rgbw {
            *rgbw = *rgbw * self.brightness;
        }
        for dimmer in &mut lights.dimmer {
            *dimmer = *dimmer * self.brightness;
        }
        for bead in &mut lights.bar.beads {
            *bead = *bead * self.brightness;
        }

        if lights.scanner1.on {
            lights.scanner1.brightness = (self.time * 16.0).square(1.0, 0.25) * 0.654;
        } else {
            lights.scanner1.brightness = 0.0;
        }

        lights.crystal0 = lights.crystal0 * self.brightness;
        lights.crystal1 *= self.brightness;
    }
    pub fn input_pad(&mut self, pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
        use launchpad_x::*;

        let page = match event {
            Input::Up(true) => Some(Page::Generators),
            Input::Down(true) => Some(Page::Presets),
            Input::Left(true) => Some(Page::Random),
            _ => None,
        };
        if let Some(page) = page {
            self.page = page;
            pad.send(Output::Clear);
            return;
        }

        match self.page {
            Page::Generators => {
                self.fx0.handle_pad(event);
                self.fx1.handle_pad(event);
            }
            Page::Presets => self.presets.handle_pad(event),
            Page::Random => self.random.handle_pad(event),
        };
    }
    pub fn input_ctrl(&mut self, event: launch_control_xl::Input) {
        println!("{event:?}");

        use launch_control_xl::*;
        match event {
            Input::Slider(7, fr) => self.brightness = fr,
            // Input::Slider(i, fr) => self.test0[i as usize] = fr,
            _ => {}
        }

        match self.page {
            Page::Generators => {
                self.fx0.handle_ctrl(event);
                self.fx1.handle_ctrl(event);
            }
            Page::Presets => self.presets.handle_ctrl(event),
            Page::Random => {}
        }
    }
    pub fn render_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};

        let on = |b| if b { Rgb::VIOLET } else { Rgb::WHITE };
        pad.send(Output::Rgb(Coord(0, 8).into(), on(matches!(self.page, Page::Generators))));
        pad.send(Output::Rgb(Coord(1, 8).into(), on(matches!(self.page, Page::Presets))));
        pad.send(Output::Rgb(Coord(2, 8).into(), on(matches!(self.page, Page::Random))));

        match self.page {
            Page::Generators => {
                self.fx0.render_pad(pad);
                self.fx1.render_pad(pad);
            }
            Page::Presets => self.presets.render_pad(pad),
            Page::Random => self.random.render_pad(pad),
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let level = match args.verbose {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    env_logger::builder()
        .filter_module("mslive", level)
        .filter_module("stagebridge", level)
        .format_timestamp(None)
        .format_module_path(false)
        .parse_default_env()
        .init();

    // Initialize input devices
    Midi::<LaunchpadX>::list()?;
    let mut pad = Midi::new("Launchpad X LPX MIDI", LaunchpadX::default());
    let mut ctrl = Midi::new("Launch Control XL", LaunchControlXL);
    {
        use launchpad_x::{types::*, *};
        pad.send(Output::Pressure(Pressure::Off, PressureCurve::Medium));
        pad.send(Output::Brightness(0.0));
    }

    // Connect to our DMX bridge
    let mut lights = Lights::new("10.16.4.1".parse()?)?;

    let mut state = State::default();

    // Start the main loop, managed by the OS's windowing system.
    let mut last = Instant::now();
    let opts = eframe::NativeOptions {
        always_on_top: true,
        initial_window_size: Some(egui::Vec2 { x: 800.0, y: 480.0 }),
        fullscreen: true,
        ..Default::default()
    };
    eframe::run_simple_native("lsd", opts, move |ctx, _frame| {
        let dt = last.elapsed().as_secs_f64();
        last = Instant::now();

        for input in ctrl.recv() {
            state.input_ctrl(input);
        }
        for input in pad.recv() {
            state.input_pad(&mut pad, input);
        }

        state.tick(dt);
        state.render_pad(&mut pad);

        lights.reset();
        state.render(&mut lights);
        lights.send();

        gui::render(&state, &lights, ctx);
        ctx.request_repaint();
    })
    .unwrap();

    Ok(())
}
