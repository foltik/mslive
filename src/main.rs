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
use std::net::UdpSocket;
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
mod logic;
mod utils;

use lights::Lights;
use logic::State;

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

    Midi::<LaunchpadX>::list();

    // Initialize prodjlink BPM rx socket
    let prodjlink = UdpSocket::bind("0.0.0.0:42069")?;
    prodjlink.set_nonblocking(true);
    let mut bpm = [0u8; 4];
    log::info!("Listening to prodjlink at {}", prodjlink.local_addr()?);

    // Initialize input devices
    // let mut pad = Midi::new("WIDI Uhost", LaunchpadX::default());
    let mut pad = Midi::new("Launchpad X LPX MIDI", LaunchpadX::default());
    let mut ctrl = Midi::new("Launch Control XL", LaunchControlXL);
    {
        use launchpad_x::{types::*, *};
        pad.send(Output::Pressure(Pressure::Off, PressureCurve::Medium));
        pad.send(Output::Brightness(0.0));
    }

    // Connect to our lighting rig's Arduino DMX adapter.
    let mut lights = Lights::new("10.16.4.1".parse()?)?;

    // Initialize main state
    let mut state = State::new();

    // Start the main loop, managed by the OS's windowing system.
    let mut last = Instant::now();
    eframe::run_simple_native("mslive", Default::default(), move |ctx, _frame| {
        let elapsed = last.elapsed();
        last = Instant::now();

        // Check for prodjlink packets
        match prodjlink.recv_from(&mut bpm) {
            Ok(_) => {
                let bpm = f32::from_le_bytes(bpm) as f64;
                log::info!("prodjlink bpm={bpm}");
                state.bpm = bpm;
            }
            _ => {}
        }

        let (s, l) = (&mut state, &mut lights);

        // Update logic
        {
            for input in ctrl.recv() {
                logic::on_ctrl(s, l, &mut ctrl, input);
            }
            for input in pad.recv() {
                logic::on_pad(s, l, &mut pad, input);
            }

            logic::tick(elapsed.as_secs_f64(), s, l);

            logic::render_lights(s, l);
            logic::render_pad(s, &mut pad);
            logic::render_ctrl(s, &mut ctrl);
        }

        // Always render the GUI each frame
        gui::render_gui(s, l, ctx);

        // Immediately request a repaint again from the OS to render at maximum speed.
        ctx.request_repaint();
    });

    Ok(())
}
