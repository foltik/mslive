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

use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;

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

    Midi::<LaunchpadX>::list();

    // Initialize input devices
    // let mut pad = Midi::new("WIDI Uhost", LaunchpadX::default());
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

    let mut stdin_channel = spawn_stdin_channel();

    // Start the main loop, managed by the OS's windowing system.
    let mut last = Instant::now();
    eframe::run_simple_native("mslive", Default::default(), move |ctx, _frame| {
        let elapsed = last.elapsed();
        last = Instant::now();

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
            if s.follow_stdin {
                logic::follow_stdin(s, &mut stdin_channel);
            }

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
fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        println!("THREAD");
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        print!("GOT {buffer}");
        tx.send(buffer).unwrap();
    });
    rx
}
