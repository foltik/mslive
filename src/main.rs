use anyhow::Result;
use clap::Parser;
use std::time::Instant;

use stagebridge::midi::device::{
    launch_control_xl::{self, LaunchControlXL},
    launchpad_x::{self, LaunchpadX},
};
use stagebridge::midi::Midi;

mod gui;
mod lights;
mod logic;

use lights::Lights;
use logic::Generator;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Log verbosity. Add more v's for more verbosity.
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,
}

pub struct State {
    fx0: Generator,
    fx1: Generator,
}

impl Default for State {
    fn default() -> Self {
        Self { fx0: Generator::new(0), fx1: Generator::new(4) }
    }
}

impl State {
    pub fn tick(&mut self, dt: f64) {
        self.fx0.tick(dt);
        self.fx1.tick(dt);
    }
    pub fn render(&self, lights: &mut Lights) {
        self.fx0.render(lights);
        self.fx1.render(lights);
    }
    pub fn input_pad(&mut self, event: launchpad_x::Input) {
        self.fx0.handle_pad(event);
        self.fx1.handle_pad(event);
    }
    pub fn input_ctrl(&mut self, event: launch_control_xl::Input) {
        self.fx0.handle_ctrl(event);
        self.fx1.handle_ctrl(event);
    }
    pub fn output_pad(&self, pad: &mut Midi<LaunchpadX>) {
        self.fx0.render_pad(pad);
        self.fx1.render_pad(pad);
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
        pad.send(Output::Brightness(1.0));
    }

    // Connect to our DMX bridge
    let mut lights = Lights::new("10.16.4.1".parse()?)?;

    let mut state = State::default();

    // Start the main loop, managed by the OS's windowing system.
    let mut last = Instant::now();
    let opts = eframe::NativeOptions {
        always_on_top: true,
        initial_window_size: Some(egui::Vec2 { x: 640.0, y: 800.0 }),
        ..Default::default()
    };
    eframe::run_simple_native("lsd", opts, move |ctx, _frame| {
        let dt = last.elapsed().as_secs_f64();
        last = Instant::now();

        for input in ctrl.recv() {
            state.input_ctrl(input);
        }
        for input in pad.recv() {
            state.input_pad(input);
        }

        state.tick(dt);
        state.output_pad(&mut pad);

        lights.reset();
        state.render(&mut lights);

        gui::render(&lights, ctx);
        ctx.request_repaint();
    })?;

    Ok(())
}
