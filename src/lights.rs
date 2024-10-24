use anyhow::Result;
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

    pub pars: [Par; 10],
    pub beams: [Beam; 4],
    pub bars: [Bar; 2],
    pub spiders: [Spider; 2],
    pub strobe: Strobe,
    pub laser: Laser,
}

impl Lights {
    pub fn new(addr: IpAddr) -> Result<Self> {
        Ok(Self {
            e131: E131::new()?,
            addr,
            pars: Default::default(),
            beams: Default::default(),
            strobe: Default::default(),
            bars: Default::default(),
            spiders: Default::default(),
            laser: Default::default(),
        })
    }

    pub fn reset(&mut self) {
        self.pars = Default::default();
        self.beams = Default::default();
        self.bars = Default::default();
        self.spiders = Default::default();
        self.strobe = Default::default();
        // self.laser = Default::default();
    }

    pub fn send(&mut self) {
        let mut dmx = [0u8; 205];

        for (i, par) in self.pars.iter().enumerate() {
            par.encode(&mut dmx[1 + 8 * i..]);
        }
        for (i, beam) in self.beams.iter().enumerate() {
            beam.encode(&mut dmx[81 + 15 * i..]);
        }
        for (i, bar) in self.bars.iter().enumerate() {
            bar.encode(&mut dmx[149 + 7 * i..]);
        }
        for (i, spider) in self.spiders.iter().enumerate() {
            spider.encode(&mut dmx[175 + 15 * i..]);
        }
        self.strobe.encode(&mut dmx[142..]);
        self.laser.encode(&mut dmx[164..]);

        self.e131.send(&self.addr, &dmx);
    }
}

impl Lights {
    /// Pars and bars one color, spiders and bars another
    pub fn split(&mut self, col0: Rgbw, col1: Rgbw) {
        self.for_each_par(|par, i, fr| par.color = col0);
        self.for_each_beam(|beam, i, fr| beam.color = col1);
        self.for_each_spider(|spider, i, fr| {
            spider.color0 = col1;
            spider.color1 = col1;
        });
        self.for_each_bar(|bar, i, fr| bar.color = col1.into());
        self.strobe.color = col0.into();
    }

    /// Apply a function to the color of each light
    pub fn map_colors(&mut self, mut f: impl FnMut(Rgbw) -> Rgbw) {
        self.for_each_par(|par, i, fr| par.color = f(par.color));
        self.for_each_beam(|beam, i, fr| beam.color = f(beam.color));
        self.for_each_spider(|spider, i, fr| {
            spider.color0 = f(spider.color0);
            spider.color1 = f(spider.color1);
        });
        self.for_each_bar(|bar, i, fr| bar.color = f(bar.color.into()).into());
        self.strobe.color = f(self.strobe.color.into()).into();
    }

    // Iterate through the lights, with additional index and fr (from 0 to 1) parameters.
    pub fn for_each_par(&mut self, f: impl FnMut(&mut Par, usize, f64)) {
        Self::for_each(&mut self.pars, f);
    }
    pub fn for_each_beam(&mut self, f: impl FnMut(&mut Beam, usize, f64)) {
        Self::for_each(&mut self.beams, f);
    }
    pub fn for_each_bar(&mut self, f: impl FnMut(&mut Bar, usize, f64)) {
        Self::for_each(&mut self.bars, f);
    }
    pub fn for_each_spider(&mut self, f: impl FnMut(&mut Spider, usize, f64)) {
        Self::for_each(&mut self.spiders, f);
    }

    fn for_each<T>(slice: &mut [T], mut f: impl FnMut(&mut T, usize, f64)) {
        let n = slice.len();
        slice.iter_mut().enumerate().for_each(|(i, t)| f(t, i, i as f64 / n as f64));
    }

    //     self.pars.fmap(|i, fr, circ, p| {
    //         p.color = match &self.par_src {
    //             Source::ParUpDown => match i {
    //                 0 => c1,
    //                 1 => c0,
    //                 2..=3 => c1,
    //                 4..=5 => c0,
    //                 6..=7 => c1,
    //                 8 => c0,
    //                 9 => c1,
    //                 _ => unreachable!(),
    //             },
    //             Source::ParSpotlight => match i {
    //                 3 => c1,
    //                 6 => c1,
    //                 _ => c0,
    //             },
    //             src => src.apply(s, c0, c1, i, fr, circ),
    //         }
    //     });

    // pub fn tick(&mut self, s: &mut State, c0: Rgbw, c1: Rgbw) {
    //     // colors
    //     self.bars.fmap(|i, fr, circ, b| b.color = self.bar_src.apply(s, c0, c1, i, fr, circ).into());
    //     self.strobe.color = self.strobe_src.apply(s, c0, c1, 0, 0.0, 0.0).into();
    //     self.beams.fmap(|i, fr, circ, b| {
    //         b.ring = self.beam_ring;
    //         b.color = self.beam_src.apply(s, c0, c1, i, fr, circ);
    //     });
    //     self.spiders.fmap(|i, fr, circ, sp| match &self.spider_src {
    //         Source::SpiderBoth => {
    //             sp.color0 = c0;
    //             sp.color1 = c1;
    //         }
    //         src => {
    //             let c = src.apply(s, c0, c1, i, fr, circ);
    //             sp.color0 = c;
    //             sp.color1 = c;
    //         }
    //     });
    //     self.pars.fmap(|i, fr, circ, p| {
    //         p.color = match &self.par_src {
    //             Source::ParUpDown => match i {
    //                 0 => c1,
    //                 1 => c0,
    //                 2..=3 => c1,
    //                 4..=5 => c0,
    //                 6..=7 => c1,
    //                 8 => c0,
    //                 9 => c1,
    //                 _ => unreachable!(),
    //             },
    //             Source::ParSpotlight => match i {
    //                 3 => c1,
    //                 6 => c1,
    //                 _ => c0,
    //             },
    //             src => src.apply(s, c0, c1, i, fr, circ),
    //         }
    //     });
}

// impl Source {
//     pub fn apply(&self, s: &State, c0: Rgbw, c1: Rgbw, i: usize, fr: f64, circ: f64) -> Rgbw {
//         match self {
//             Source::Off => Rgbw::BLACK,
//             Source::C0 => c0,
//             Source::C1 => c1,
//             Source::Alternate => match i % 2 == 0 {
//                 true => c0,
//                 false => c1,
//             },
//             Source::Strobe { pd, duty } => c0 * (s.pd(*pd).square(1.0, *duty)),
//             Source::Chase { pd, duty } => c0 * (s.pd(*pd).phase(1.0, circ).square(1.0, *duty)),

//             // these do nothing here, special cased by their respective lights
//             Source::SpiderBoth => unreachable!(),
//             Source::ParUpDown => unreachable!(),
//             Source::ParSpotlight => unreachable!(),
//         }
//     }
// }
