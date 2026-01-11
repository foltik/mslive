#![allow(unused)]

use stagebridge::midi::device::{
    launch_control_xl::{self, LaunchControlXL},
    launchpad_x::{self, LaunchpadX},
};
use stagebridge::midi::Midi;

use crate::{lights::Lights, pages::Page};

#[derive(Default)]
pub struct Test {
    sliders: [f64; 8],
    toggles: [bool; 8],
}

impl Page for Test {
    fn tick(&mut self, dt: f64) {}

    fn input_pad(&mut self, _pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input) {
        use launchpad_x::{types::*, *};
        log::info!("{event:?}");
    }

    fn input_ctrl(&mut self, event: launch_control_xl::Input) {
        use launch_control_xl::{types::*, *};
        log::info!("{event:?}");

        match event {
            Input::Slider(i, fr) => self.sliders[i as usize] = fr,
            Input::Control(i, true) => self.toggles[i as usize] = !self.toggles[i as usize],
            _ => {}
        }
    }

    fn output_lights(&self, lights: &mut Lights) {}

    fn output_pad(&self, pad: &mut Midi<LaunchpadX>) {
        use launchpad_x::{types::*, *};
    }

    fn output_ctrl(&self, ctrl: &mut Midi<LaunchControlXL>) {
        use launch_control_xl::{types::*, *};
        for i in 0..8 {
            ctrl.send(Output::Focus(i, Color::Red, Brightness::High));
            ctrl.send(Output::Control(
                i,
                Color::Amber,
                if self.toggles[i as usize] { Brightness::High } else { Brightness::Off },
            ));
        }
    }
}
