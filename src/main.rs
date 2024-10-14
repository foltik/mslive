#![allow(unused)]
#![allow(clippy::single_match)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::needless_pass_by_ref_mut)]
#![feature(stmt_expr_attributes)]

use anyhow::Result;
use clap::{ArgAction, Parser};
use itertools::Itertools;
use rand::seq::SliceRandom;
use stagebridge::color::{Rgb, Rgbw};
use stagebridge::dmx::device::laser_scan_30w::{LaserColor, LaserPattern};
use std::time::Instant;
use std::{thread, time::Duration};

use stagebridge::e131::E131;
use stagebridge::midi::device::{
    launch_control_xl::{self, LaunchControlXL},
    launchpad_x::{self, LaunchpadX},
};
use stagebridge::midi::Midi;
use stagebridge::prelude::*;

mod gui;
mod lights;
mod state;
mod utils;

use lights::Lights;
use state::State;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Log verbosity. Add more v's for more verbosity.
    #[arg(short, action = ArgAction::Count)]
    verbose: u8,
}

fn main() -> Result<()> {
    // Set up colorful logging for `log::` calls.
    let args = Args::parse();
    env_logger::builder()
        .filter_module(
            "mslive",
            match args.verbose {
                0 => log::LevelFilter::Info,
                1 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            },
        )
        .format_timestamp(None)
        .format_module_path(false)
        .parse_default_env()
        .init();

    // Initialize input devices
    let mut pad = Midi::new("Launchpad X:Launchpad X LPX MIDI", LaunchpadX::default());
    let mut ctrl = Midi::new("Launch Control XL:Launch Control XL", LaunchControlXL);
    {
        use launchpad_x::{types::*, *};
        pad.send(Output::Pressure(Pressure::Off, PressureCurve::Medium));
        pad.send(Output::Brightness(1.0));
    }

    // Connect to our lighting rig's Arduino DMX adapter.
    let mut lights = Lights::new("10.16.4.1".parse()?)?;

    // Initialize main state
    let mut state = State::new();

    // Start the main loop, managed by the OS's windowing system.
    let mut last = Instant::now();
    eframe::run_simple_native("mslive", Default::default(), move |ctx, _frame| {
        // Immediately request a repaint again from the OS to render at maximum speed.
        ctx.request_repaint();

        // Limit the framerate to 200fps
        let elapsed = last.elapsed();
        if elapsed < Duration::from_millis(5) {
            return;
        }
        last = Instant::now();

        // Main update logic
        {
            let s = &mut state;
            let l = &mut lights;

            for input in ctrl.recv() {
                state::on_ctrl(s, l, input);
            }
            for input in pad.recv() {
                state::on_pad(s, l, input);
            }

            state::tick(elapsed.as_secs_f64(), s, l);

            state::render_lights(s, l);
            state::render_pad(s, &mut pad);
            state::render_ctrl(s, &mut ctrl);
            gui::render_gui(s, l, ctx);
        }
    });

    Ok(())
}
