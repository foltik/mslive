use stagebridge::midi::{
    device::{
        launch_control_xl::{self, LaunchControlXL},
        launchpad_x::{self, LaunchpadX},
    },
    Midi,
};

use crate::lights::Lights;

mod generators;
mod presets;
mod random;
mod test;

pub trait Page {
    fn tick(&mut self, dt: f64);
    fn input_pad(&mut self, pad: &mut Midi<LaunchpadX>, event: launchpad_x::Input);
    fn input_ctrl(&mut self, event: launch_control_xl::Input);

    fn output_lights(&self, lights: &mut Lights);
    fn output_pad(&self, pad: &mut Midi<LaunchpadX>);
    fn output_ctrl(&self, pad: &mut Midi<LaunchControlXL>);
}

pub fn pages() -> Vec<Box<dyn Page>> {
    vec![
        Box::new(generators::Generators::default()),
        Box::new(test::Test::default()),
        //
    ]
}
