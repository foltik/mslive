use anyhow::Result;
use std::net::IpAddr;

use stagebridge::color::Rgbw;
use stagebridge::e131::E131;
use stagebridge::prelude::*;

pub struct Lights {
    e131: E131,
    addr: IpAddr,

    pub dimmer: [f64; 9],
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

    pub fn send(&mut self) {
        let mut dmx = [0u8; 110];

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
        for i in 0..9 {
            dmx[100 + i] = self.dimmer[i].byte();
        }

        self.e131.send(&self.addr, &dmx);
    }
}
