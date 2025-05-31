use anyhow::Result;
use stagebridge::dmx::device::beam_rgbw_90w::BigBeam;
use std::net::IpAddr;

use stagebridge::color::Rgbw;
use stagebridge::dmx::device::bar_rgb_18w::Bar;
use stagebridge::dmx::device::beam_rgbw_60w::{Beam, BeamRing};
use stagebridge::dmx::device::laser_scan_30w::{Laser, LaserColor};
use stagebridge::dmx::device::par_rgbw_12x3w::Par;
use stagebridge::dmx::device::spider_rgbw_8x10w::Spider;
use stagebridge::dmx::device::strobe_rgb_35w::Strobe;
use stagebridge::dmx::Device;
use stagebridge::e131::E131;
use stagebridge::prelude::*;

use crate::utils::Pd;
use crate::State;

pub struct Lights {
    e131: E131,
    addr: IpAddr,

    pub dimmer: [f64; 8],
    pub rgbw: [Rgbw; 9],
}

impl Lights {
    pub fn new(addr: IpAddr) -> Result<Self> {
        Ok(Self {
            e131: E131::new()?,
            addr,
            rgbw: [Rgbw::BLACK; 9],
            dimmer: Default::default(),
        })
    }

    pub fn reset(&mut self) {
        self.dimmer = Default::default();
        self.rgbw = Default::default();
    }

    pub fn send(&mut self, chan: usize) {
        let mut dmx = [0u8; 120];

        // RGBW bars
        macro_rules! rgbw {
            ($self:expr, $dmx:expr, $chan:expr, $i:expr) => {
                dmx[$chan + 0] = $self.rgbw[$i].0.byte();
                dmx[$chan + 1] = $self.rgbw[$i].1.byte();
                dmx[$chan + 2] = $self.rgbw[$i].2.byte();
                dmx[$chan + 3] = $self.rgbw[$i].3.byte();
            };
        }
        rgbw!(self, dmx, 1, 0);
        rgbw!(self, dmx, 5, 1);
        rgbw!(self, dmx, 9, 2);
        rgbw!(self, dmx, 13, 3);
        rgbw!(self, dmx, 21, 4);
        rgbw!(self, dmx, 25, 5);
        rgbw!(self, dmx, 37, 6);
        rgbw!(self, dmx, 49, 7);
        rgbw!(self, dmx, 53, 8);

        // Dimmer packs
        for i in 0..8 {
            dmx[100 + i] = self.dimmer[i].byte();
        }

        // self.e131.send(&self.addr, &dmx);
    }
}

impl Lights {
    fn for_each<T>(slice: &mut [T], mut f: impl FnMut(&mut T, usize, f64)) {
        let n = slice.len();
        slice.iter_mut().enumerate().for_each(|(i, t)| f(t, i, i as f64 / n as f64));
    }

    // Mutably iterate through the lights, with index and fr (from 0 to 1) parameters.

    pub fn for_each_dimmer(&mut self, f: impl FnMut(&mut f64, usize, f64)) {
        Self::for_each(&mut self.dimmer, f);
    }
    pub fn for_reach_rgbw(&mut self, f: impl FnMut(&mut Rgbw, usize, f64)) {
        Self::for_each(&mut self.rgbw, f);
    }
}
