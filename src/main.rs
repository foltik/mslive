use std::{time::Duration, thread};
use color_eyre::Result;

use stagebridge::{midi::{Midi, device::launchpad_x::LaunchpadX}, num::Float};

// mod lights; use lights::*;
// mod logic; use logic::*;
mod state; use state::*;
mod color; use color::*;
mod fx;

fn main() -> Result<()> {
    color_eyre::install()?;
    pretty_env_logger::init();

    let mut pad = Midi::connect("Launchpad X:Launchpad X LPX MIDI", LaunchpadX::default())?;
    {
        use stagebridge::midi::device::launchpad_x::{*, types::*};
        pad.send(Output::Mode(Mode::Programmer));
        pad.send(Output::Pressure(Pressure::Off, PressureCurve::Medium));
        pad.send(Output::Clear);

    }

    let mut s = State::default();

    loop {
        thread::sleep(Duration::from_millis(5));
    }
}

impl State {
    pub fn phi(&self, pd: Pd) -> f64 {
        self.phi.mod_div(pd.fr() * self.phi_mul)
    }

    pub fn color0(&self) -> Color {
        self.map0.apply(self, self.color0.apply(self))
    }
    pub fn color1(&self) -> Color {
        self.map1.apply(self, self.color1.apply(self))
    }

    pub fn color0_phase(&self, pd: Pd, offset: f64) -> Color {
        let mut state = self.clone();
        state.phi = state.phi.phase(pd.fr(), offset);
        self.map0.apply(&state, self.color0.apply(&state))
    }
    pub fn color1_phase(&self, pd: Pd, offset: f64) -> Color {
        let mut state = self.clone();
        state.phi = state.phi.phase(pd.fr(), offset);
        self.map1.apply(&state, self.color1.apply(&state))
    }
}
