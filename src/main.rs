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
mod gui;
mod lights;
mod pages;
mod utils;

use lights::Lights;
use pages::Page;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Log verbosity. Add more v's for more verbosity.
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,
}

pub struct State {
    pages: Vec<Box<dyn Page>>,
    page: usize,

    brightness: f64,
    brightness_ceil: f64,
    time: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pages: pages::pages(),
            page: 0,
            brightness: 1.0,
            brightness_ceil: 1.0,
            time: 0.0,
        }
    }
}

impl State {
    pub fn tick(&mut self, dt: f64) {
        self.time += dt;
        self.pages[self.page].tick(dt);
    }

    pub fn output_lights(&self, lights: &mut Lights) {
        self.pages[self.page].output_lights(lights);

        // Apply some post-processing on the page's outputs
        {
            // Adjust global brightness
            let brightness_fr = self.brightness * self.brightness_ceil;
            for rgbw in &mut lights.rgbw {
                *rgbw *= brightness_fr;
            }
            for dimmer in &mut lights.dimmer {
                *dimmer *= brightness_fr;
            }
            for bead in &mut lights.bar.beads {
                *bead *= brightness_fr;
            }
            lights.crystal0 *= brightness_fr;
            lights.crystal1 *= brightness_fr;

            // The brightness on scanner1 doesn't work properly, so we apply PWM to make it dimmer
            if lights.scanner1.on {
                // The
                lights.scanner1.brightness = (self.time * 16.0).square(1.0, 0.25) * 0.654;
            } else {
                lights.scanner1.brightness = 0.0;
            }
        }
    }

    pub fn input_pad(&mut self, pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) -> Option<usize> {
        use launchpad_x::*;

        // Page switching
        let page = match event {
            Input::Up(true) => Some(0),
            Input::Down(true) => Some(1),
            Input::Left(true) => Some(2),
            Input::Right(true) => Some(3),
            Input::Session(true) => Some(4),
            Input::Note(true) => Some(5),
            Input::Custom(true) => Some(6),
            Input::Capture(true) => Some(7),
            _ => None,
        };

        self.pages[self.page].input_pad(pad, event);

        page
    }
    pub fn input_ctrl(&mut self, event: launch_control_xl::Input) {
        // Global brightness control
        use launch_control_xl::*;
        match event {
            Input::SendA(0, fr) => self.brightness = (fr + 1.0) / 2.0,
            Input::SendA(1, fr) => self.brightness_ceil = (fr + 1.0) / 2.0,
            _ => {}
        }

        self.pages[self.page].input_ctrl(event);
    }
    pub fn output_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};

        // Highlight current page
        for i in 0..8usize {
            pad.send(Output::Rgb(
                Coord(i as i8, 8).into(),
                match i {
                    _ if self.page == i => Rgb::VIOLET,
                    _ if i < self.pages.len() => Rgb::WHITE,
                    _ => Rgb::BLACK,
                },
            ));
        }

        self.pages[self.page].output_pad(pad);
    }

    pub fn output_ctrl(&self, ctrl: &mut Midi<LaunchControlXL>) {
        use launch_control_xl::{types::*, *};

        // Highlight brightness knobs
        ctrl.send(Output::SendA(0, Color::Red, Brightness::High));
        ctrl.send(Output::SendA(1, Color::Red, Brightness::High));

        self.pages[self.page].output_ctrl(ctrl);
    }
}

fn clear(pad: &mut Midi<LaunchpadX>, ctrl: &mut Midi<LaunchControlXL>) {
    {
        use launchpad_x::*;
        pad.send(Output::Clear);
    }
    {
        use launch_control_xl::{types::*, *};
        for i in 0..8 {
            ctrl.send(Output::SendA(i, Color::Red, Brightness::Off));
            ctrl.send(Output::SendB(i, Color::Red, Brightness::Off));
            ctrl.send(Output::Pan(i, Color::Red, Brightness::Off));
            ctrl.send(Output::Focus(i, Color::Red, Brightness::Off));
            ctrl.send(Output::Control(i, Color::Red, Brightness::Off));
        }
        ctrl.send(Output::SendSelect(true, Color::Red, Brightness::Off));
        ctrl.send(Output::SendSelect(false, Color::Red, Brightness::Off));
        ctrl.send(Output::TrackSelect(true, Color::Red, Brightness::Off));
        ctrl.send(Output::TrackSelect(false, Color::Red, Brightness::Off));
        ctrl.send(Output::Device(Color::Red, Brightness::Off));
        ctrl.send(Output::Mute(Color::Red, Brightness::Off));
        ctrl.send(Output::Solo(Color::Red, Brightness::Off));
        ctrl.send(Output::Record(Color::Red, Brightness::Off));
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
    let mut ctrl = Midi::new("Launch Control XL", LaunchControlXL::default());
    {
        use launchpad_x::{types::*, *};
        pad.send(Output::Pressure(Pressure::Off, PressureCurve::Medium));
        pad.send(Output::Brightness(1.0));
    }

    // Connect to our DMX bridge
    let mut lights = Lights::new("10.16.4.1".parse()?)?;

    let mut state = State::default();

    // Start the main loop, managed by the OS's windowing system.
    let mut last = Instant::now();
    let opts = eframe::NativeOptions {
        always_on_top: true,
        initial_window_size: Some(egui::Vec2 { x: 800.0, y: 480.0 }),
        fullscreen: false,
        ..Default::default()
    };
    eframe::run_simple_native("lsd", opts, move |ctx, _frame| {
        let dt = last.elapsed().as_secs_f64();
        last = Instant::now();

        for input in pad.recv() {
            if let Some(new_page) = state.input_pad(&mut pad, input) {
                if new_page < state.pages.len() {
                    state.page = new_page;
                    clear(&mut pad, &mut ctrl);
                }
            }
        }
        for input in ctrl.recv() {
            state.input_ctrl(input);
        }

        state.tick(dt);
        state.output_pad(&mut pad);
        state.output_ctrl(&mut ctrl);

        lights.reset();
        state.output_lights(&mut lights);
        lights.send();

        gui::render(&state, &lights, ctx);
        ctx.request_repaint();
    })
    .unwrap();

    Ok(())
}
