use stagebridge::color::Rgbw;
use stagebridge::dmx::Device;
use stagebridge::dmx::device::beam_rgbw_60w::{Beam, BeamRing};
use stagebridge::dmx::device::laser_scan_30w::{Laser, LaserColor};
use stagebridge::dmx::device::spider_rgbw_8x10w::Spider;
use stagebridge::dmx::device::strobe_rgb_35w::Strobe;
use stagebridge::dmx::device::par_rgbw_12x3w::Par;
use stagebridge::dmx::device::bar_rgb_18w::Bar;
use stagebridge::e131::E131;
use stagebridge::num::Float;

use crate::State;
use crate::types::{Pd, RgbwExt};

#[derive(Clone, Default)]
pub struct Lights {
    pub pars: [Par; 10],
    pub par_src: Source,

    pub beams: [Beam; 4],
    pub beam_src: Source,
    pub beam_pos: BeamPos,
    pub beam_ring: BeamRing,

    pub strobe: Strobe,
    pub strobe_src: Source,

    pub bars: [Bar; 2],
    pub bar_src: Source,

    pub spiders: [Spider; 2],
    pub spider_src: Source,
    pub spider_pos: SpiderPos,

    pub laser: Laser,
    pub laser_pos: LaserPos,
}

#[derive(Clone, Default)]
pub enum Source {
    Off,
    #[default]
    C0,
    C1,
    Alternate,
    Strobe { pd: Pd, duty: f64 },
    Chase { pd: Pd, duty: f64 },

    SpiderBoth,

    ParUpDown,
    ParSpotlight,
    // Fade,
    // Alternate { pd: Pd },
}

#[derive(Clone, Default)]
pub enum BeamPos {
    #[default]
    Down,
    Out,
    SpreadOut,
    SpreadIn,
    Cross,
    CrissCross,
    WaveY { pd: Pd },
    SnapX { pd: Pd },
    SnapY { pd: Pd },
    Square { pd: Pd },
}

#[derive(Clone, Copy, Debug, Default)]
pub enum SpiderPos {
    Up,
    #[default]
    Down,
    Wave { pd: Pd },
    Alternate { pd: Pd },
    Snap { pd: Pd },
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LaserPos {
    #[default]
    Still,
    Rotate { pd: Pd },
    WaveY { pd: Pd },
}

impl Lights {
    pub fn tick(&mut self, s: &mut State, c0: Rgbw, c1: Rgbw) {
        // colors
        self.bars.fmap(|i, fr, circ, b| b.color = self.bar_src.apply(s, c0, c1, i, fr, circ).into());
        self.strobe.color = self.strobe_src.apply(s, c0, c1, 0, 0.0, 0.0).into();
        self.beams.fmap(|i, fr, circ, b| {
            b.ring = self.beam_ring;
            b.color = self.beam_src.apply(s, c0, c1, i, fr, circ);
        });
        self.spiders.fmap(|i, fr, circ, sp| match &self.spider_src {
            Source::SpiderBoth => {
                sp.color0 = c0;
                sp.color1 = c1;
            },
            src => {
                let c = src.apply(s, c0, c1, i, fr, circ);
                sp.color0 = c;
                sp.color1 = c;
            },
        });
        self.pars.fmap(|i, fr, circ, p| p.color = match &self.par_src {
            Source::ParUpDown => match i {
                0 => c1,
                1 => c0,
                2..=3 => c1,
                4..=5 => c0,
                6..=7 => c1,
                8 => c0,
                9 => c1,
                _ => unreachable!(),
            },
            Source::ParSpotlight => match i {
                3 => c1,
                6 => c1,
                _ => c0,
            },
            src => src.apply(s, c0, c1, i, fr, circ),
        });

        // laser pos
        match self.laser_pos {
            LaserPos::Rotate { pd } => self.laser.rotate = s.pd(pd),
            LaserPos::WaveY { pd } => self.laser.y = s.pd(pd),
            LaserPos::Still => {},
        }

        // spider pos
        self.spiders.fmap(|i, fr, circ, sp| {
            match self.spider_pos {
                SpiderPos::Up => {
                    sp.pos0 = 0.0;
                    sp.pos1 = 0.52;
                },
                SpiderPos::Down => {
                    sp.pos0 = 0.67;
                    sp.pos1 = 0.52;
                },
                SpiderPos::Wave { pd } => {
                    let fr = s.pd(pd.mul(2)).tri(1.0);
                    sp.pos0 = fr;
                    sp.pos1 = 1.0 - fr;
                },
                SpiderPos::Alternate { pd } => {
                    let t = s.pd(pd.mul(2));
                    let t = match i {
                        0 => t,
                        1 => t.phase(1.0, 0.5),
                        _ => unreachable!(),
                    };
                    let fr = t.tri(1.0);
                    sp.pos0 = fr;
                    sp.pos1 = fr;
                },
                SpiderPos::Snap { pd } => {
                    let t = s.pd(pd.mul(2));
                    let t = match i {
                        0 => t,
                        1 => t.phase(1.0, 0.5),
                        _ => unreachable!(),
                    };
                    let fr = t.square(1.0, 0.5);
                    sp.pos0 = fr;
                    sp.pos1 = fr;
                },
            }
        });

        // global beam pos
        match self.beam_pos {
            BeamPos::SpreadOut => {
                self.beams[0].yaw = 0.5 - 0.05;
                self.beams[1].yaw = 0.5 - 0.02;
                self.beams[2].yaw = 0.5 + 0.02;
                self.beams[3].yaw = 0.5 + 0.05;
            },
            BeamPos::SpreadIn => {
                self.beams[0].yaw = 0.5 + 0.09;
                self.beams[1].yaw = 0.5 + 0.07;
                self.beams[2].yaw = 0.5 - 0.07;
                self.beams[3].yaw = 0.5 - 0.09;
            },
            BeamPos::Cross => {
                self.beams[0].yaw = 0.5 + 0.13;
                self.beams[1].yaw = 0.5 + 0.13;
                self.beams[2].yaw = 0.5 - 0.13;
                self.beams[3].yaw = 0.5 - 0.13;
            },
            BeamPos::CrissCross => {
                self.beams[0].yaw = 0.5 + 0.08;
                self.beams[1].yaw = 0.5 - 0.05;
                self.beams[2].yaw = 0.5 + 0.05;
                self.beams[3].yaw = 0.5 - 0.08;
            },
            _ => {},
        };

        // per-beam pos
        self.beams.fmap(|i, fr, circ, b| match self.beam_pos {
            BeamPos::Down => {
                b.pitch = 0.0;
                b.yaw = 0.5;
            },
            BeamPos::Out => {
                b.pitch = 0.5;
                b.yaw = 0.5;
            },
            BeamPos::SnapY { pd } => {
                b.yaw = 0.5;
                let t = s.pd(pd.mul(4)).square(1.0, 0.5);
                b.pitch = 0.3 * match i % 2 == 0 {
                    true => t,
                    false => 1.0 - t,
                }
            }
            BeamPos::SnapX { pd } => {
                let t = s.pd(pd.mul(4)).negsquare(1.0, 0.5);
                b.yaw = 0.5 + 0.13 * match i > 1 {
                    true => t,
                    false => -t,
                };
                b.pitch = 0.3 * s.pd(pd.mul(2)).square(1.0, 0.5);
            },
            BeamPos::WaveY { pd } => {
                b.yaw = 0.5;
                let t = s.pd(pd.mul(2)).tri(1.0);
                b.pitch = 0.3 * match i % 2 == 0 {
                    true => t,
                    false => 1.0 - t,
                }
            }
            BeamPos::Square { pd } => {
                let t_pitch = s.pd(pd.mul(4)).phase(1.0, 0.25).square(1.0, 0.5);
                let t_yaw = match i % 2 == 0 {
                    true => s.pd(pd.mul(4)).negsquare(1.0, 0.5),
                    false => s.pd(pd.mul(4)).phase(1.0, 0.5).negsquare(1.0, 0.5)
                };
                b.pitch = 0.1 + 0.25 * match i % 2 == 0 {
                    true => t_pitch,
                    false => 1.0 - t_pitch,
                };
                b.yaw = 0.5 + 0.08 * t_yaw;
            }
            _ => {}
        });
    }
}

impl Source {
    pub fn apply(&self, s: &State, c0: Rgbw, c1: Rgbw, i: usize, fr: f64, circ: f64) -> Rgbw {
        match self {
            Source::Off => Rgbw::BLACK,
            Source::C0 => c0,
            Source::C1 => c1,
            Source::Alternate => match i % 2 == 0 {
                true => c0,
                false => c1,
            },
            Source::Strobe { pd, duty } => c0.a(s.pd(*pd).square(1.0, *duty)),
            Source::Chase { pd, duty } => c0.a(s.pd(*pd).phase(1.0, circ).square(1.0, *duty)),

            // these do nothing here, special cased by their respective lights
            Source::SpiderBoth => unreachable!(),
            Source::ParUpDown => unreachable!(),
            Source::ParSpotlight => unreachable!(),
        }
    }
}

impl Lights {
    pub fn send(&self, e131: &mut E131) {
        let mut dmx = [0u8; 205];

        for (i, par) in self.pars.iter().enumerate() {
            par.encode(&mut dmx[1 + 8*i..]);
        }

        for (i, beam) in self.beams.iter().enumerate() {
            beam.encode(&mut dmx[81 + 15 * i..]);
        }

        self.strobe.encode(&mut dmx[142..]);

        for (i, bar) in self.bars.iter().enumerate() {
            bar.encode(&mut dmx[149 + 7 * i..]);
        }

        self.laser.encode(&mut dmx[164..]);

        for (i, spider) in self.spiders.iter().enumerate() {
            spider.encode(&mut dmx[175 + 15*i..]);
        }

        e131.send(&dmx);
    }

    pub fn paint(&self, p: &egui::Painter, w0: f64, h0: f64) {
        // bounds
        let w = w0 * 0.8;
        let h = h0 * 0.8;
        let x0 = (w0 - w) * 0.5;
        let y0 = (h0 - h) * 0.5;

        // table
        Self::rect(p, Rgbw::BLACK, x0 + w*0.5, y0 + h*0.8, 350.0, 75.0);

        // pars
        let dy = w / 9.0;
        &self.pars[1..=8].feach(|i, fr, circ, par| {
            Self::circle(p, par.color, x0 + w*(1.0 - fr), y0, 10.0);
        });
        Self::circle(p, self.pars[0].color, x0,     y0 + dy, 10.0);
        Self::circle(p, self.pars[9].color, x0 + w, y0 + dy, 10.0);

        // bars
        Self::rect(p, self.bars[0].color.into(), x0 + w*0.166, y0 + h*0.93, 100.0, 15.0);
        Self::rect(p, self.bars[1].color.into(), x0 + w*0.833, y0 + h*0.93, 100.0, 15.0);
        Self::rect(p, self.strobe.color.into(), x0 + w*0.5, y0 + h*0.93, 80.0, 25.0);

        // beams
        let bw = w * 0.7;
        let bx0 = x0 + (w - bw) * 0.5;
        self.beams.feach(|i, fr, circ, beam| {
            let dx = beam.yaw.ssin(1.0) * 10.0;
            let dy = beam.pitch.ssin(1.0) * 10.0;
            Self::rect(p, Rgbw::BLACK, bx0 + bw*(1.0-fr), y0 + 0.1*h, 20.0, 20.0);
            Self::circle(p, beam.color, bx0 + bw*(1.0-fr) + dx, y0 + 0.1*h + dy, 6.0);
        });

        // spiders
        let sw = w * 0.48;
        let sx0 = x0 + (w - sw) * 0.5;
        self.spiders.feach(|i, fr, circ, spider| {
            let dy0 = spider.pos0 * 4.0;
            let dy1 = -spider.pos1 * 4.0;

            Self::rect(p, Rgbw::BLACK, sx0 + sw*(1.0-fr), y0 + 0.17*h, 50.0, 20.0);
            for j in 0..4 {
                let jfr = j as f64 / 3.0;
                Self::rect(p, spider.color0, sx0 + sw*(1.0-fr) + jfr*35.0 - 17.5, y0 + 0.162*h + dy0, 5.0, 5.0);
                Self::rect(p, spider.color1, sx0 + sw*(1.0-fr) + jfr*35.0 - 17.5, y0 + 0.178*h + dy1, 5.0, 5.0);
            }
        })
    }

    fn circle(p: &egui::Painter, c: Rgbw, x: f64, y: f64, r: f64) {
        p.circle_filled(egui::Pos2::new(x as f32, y as f32), r as f32, c.e());
    }

    fn rect(p: &egui::Painter, c: Rgbw, x: f64, y: f64, w: f64, h: f64) {
        let rect = egui::Rect::from_center_size(egui::Pos2::new(x as f32, y as f32), egui::Vec2::new(w as f32, h as f32));
        p.rect_filled(rect, egui::Rounding::ZERO, c.e());
    }

    fn line(p: &egui::Painter, c: Rgbw, x0: f64, y0: f64, x1: f64, y1: f64) {

    }
}

trait Map<T> {
    fn fmap<F: FnMut(usize, f64, f64, &mut T)>(&mut self, f: F);
    fn feach<F: FnMut(usize, f64, f64, &T)>(&self, f: F);
}

impl<T> Map<T> for [T] {
    fn fmap<F: FnMut(usize, f64, f64, &mut T)>(&mut self, mut f: F) {
        let n = self.len();
        for (i, t) in self.iter_mut().enumerate() { f(i, i as f64 / (n - 1) as f64, i as f64 / n as f64, t); }
    }
    fn feach<F: FnMut(usize, f64, f64, &T)>(&self, mut f: F) {
        let n = self.len();
        for (i, t) in self.iter().enumerate() { f(i, i as f64 / (n - 1) as f64, i as f64 / n as f64, t); }
    }
}
