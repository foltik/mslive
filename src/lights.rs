use anyhow::Result;
use std::net::IpAddr;

use stagebridge::e131::E131;
use stagebridge::{color::Rgbw, dmx::Device};

use stagebridge::prelude::*;

use crate::dmx::{Bar, Scanner1, Scanner2};

pub struct Lights {
    e131: E131,
    addr: IpAddr,

    pub dimmer: [f64; 9],
    pub rgbw: [Rgbw; 9],
    pub scanner1: Scanner1,
    pub scanner2: Scanner2,
    pub crystal0: Rgb,
    pub crystal1: f64,
    pub bar: Bar,

    pub test0: [u8; 8],
    pub test1: [u8; 8],
}

impl Lights {
    pub fn new(addr: IpAddr) -> Result<Self> {
        Ok(Self {
            e131: E131::new()?,
            addr,
            rgbw: [Rgbw::BLACK; 9],
            dimmer: Default::default(),
            scanner1: Scanner1::default(),
            scanner2: Scanner2::default(),
            bar: Bar::default(),
            crystal1: 0.0,
            crystal0: Rgb::BLACK,
            test0: [0; 8],
            test1: [0; 8],
        })
    }

    pub fn reset(&mut self) {
        self.dimmer = Default::default();
        self.rgbw = Default::default();
        self.bar = Default::default();
        self.scanner1 = Default::default();
        self.scanner2 = Default::default();
        self.crystal0 = Default::default();
        self.crystal1 = 0.0;
    }

    pub fn send(&mut self) {
        let mut dmx = [0u8; 200];

        // RGBW bars
        macro_rules! rgbw {
            ($self:expr, $dmx:expr, $chan:expr, $i:expr) => {
                dmx[$chan + 0] = $self.rgbw[$i].0.byte();
                dmx[$chan + 1] = $self.rgbw[$i].1.byte();
                dmx[$chan + 2] = $self.rgbw[$i].2.byte();
                dmx[$chan + 3] = $self.rgbw[$i].3.byte();
            };
        }

        // let chan = 8;
        // for i in 0..4 {
        //     rgbw!(self, dmx, (3 * i) + chan, i);
        // }

        rgbw!(self, dmx, 8, 0);
        rgbw!(self, dmx, 12, 1);
        rgbw!(self, dmx, 16, 2);
        rgbw!(self, dmx, 20, 3);

        rgbw!(self, dmx, 32, 4);
        rgbw!(self, dmx, 36, 5);
        rgbw!(self, dmx, 40, 6);
        rgbw!(self, dmx, 44, 7);
        rgbw!(self, dmx, 48, 8);

        // rgbw!(self, dmx, 1, 0);
        // rgbw!(self, dmx, 5, 1);
        // rgbw!(self, dmx, 9, 2);
        // rgbw!(self, dmx, 13, 3);
        // rgbw!(self, dmx, 21, 4);
        // rgbw!(self, dmx, 25, 5);
        // rgbw!(self, dmx, 37, 6);
        // rgbw!(self, dmx, 49, 7);
        // rgbw!(self, dmx, 53, 8);

        dmx[47] = self.crystal1.byte();
        dmx[48] = if self.crystal1 > 0.0 { 255 } else { 0 };

        // Mini crystal spot
        dmx[60] = self.crystal0.0.byte();
        dmx[61] = self.crystal0.1.byte();
        dmx[62] = self.crystal0.2.byte();

        // Dimmer packs
        for i in 0..9 {
            dmx[100 + i] = self.dimmer[i].byte();
        }

        // for i in 0..200 {
        //     dmx[i] = 0;
        // }

        // Scanners
        self.scanner1.encode(&mut dmx[1..]);
        // self.scanner1.encode(&mut dmx[64..]);
        // self.scanner1.encode(&mut dmx[128..]);
        // self.scanner2.encode(&mut dmx[136..]);

        // Light bar
        self.bar.encode(&mut dmx[146..]);

        println!("{:?}", &dmx[1..=(1 + 8)]);
        // println!("{:?}", &dmx[128..=(128 + 8)]);
        self.e131.send(&self.addr, &dmx);
    }
}
