use stagebridge::color::{Rgb, Rgbw};
use stagebridge::num::Interp;

use crate::lights::Lights;
use crate::logic::State;

pub fn render_gui(s: &State, l: &Lights, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let size = ui.available_size();
        let (resp, painter) = ui.allocate_painter(size, egui::Sense::hover());
        draw_inner(s, l, &painter, size.x as f64, size.y as f64);
    });
}

fn draw_inner(s: &State, l: &Lights, p: &egui::Painter, w0: f64, h0: f64) {
    // bounds
    let w = w0 * 0.8;
    let h = h0 * 0.8;
    let x0 = (w0 - w) * 0.5;
    let y0 = (h0 - h) * 0.5;

    // table
    rect(p, Rgbw::BLACK, x0 + w * 0.5, y0 + h * 0.8, 350.0, 75.0);

    // pars
    let dy = w / 9.0;
    &l.pars[1..=8].feach(|i, fr, circ, par| {
        circle(p, par.color, x0 + w * (1.0 - fr), y0, 10.0);
    });
    circle(p, l.pars[0].color, x0, y0 + dy, 10.0);
    circle(p, l.pars[9].color, x0 + w, y0 + dy, 10.0);

    // bars
    rect(p, l.bars[0].color.into(), x0 + w * 0.166, y0 + h * 0.93, 100.0, 15.0);
    rect(p, l.bars[1].color.into(), x0 + w * 0.833, y0 + h * 0.93, 100.0, 15.0);
    rect(p, l.strobe.color.into(), x0 + w * 0.5, y0 + h * 0.93, 80.0, 25.0);

    // beams
    let bw = w * 0.7;
    let bx0 = x0 + (w - bw) * 0.5;
    l.beams.feach(|i, fr, circ, beam| {
        let dx = beam.yaw.ssin(1.0) * 10.0;
        let dy = beam.pitch.ssin(1.0) * 10.0;
        rect(p, Rgbw::BLACK, bx0 + bw * (1.0 - fr), y0 + 0.1 * h, 20.0, 20.0);
        circle(p, beam.color, bx0 + bw * (1.0 - fr) + dx, y0 + 0.1 * h + dy, 6.0);
    });

    // spiders
    let sw = w * 0.48;
    let sx0 = x0 + (w - sw) * 0.5;
    l.spiders.feach(|i, fr, circ, spider| {
        let dy0 = spider.pos0 * 4.0;
        let dy1 = -spider.pos1 * 4.0;

        rect(p, Rgbw::BLACK, sx0 + sw * (1.0 - fr), y0 + 0.17 * h, 50.0, 20.0);
        for j in 0..4 {
            let jfr = j as f64 / 3.0;
            rect(p, spider.color0, sx0 + sw * (1.0 - fr) + jfr * 35.0 - 17.5, y0 + 0.162 * h + dy0, 5.0, 5.0);
            rect(p, spider.color1, sx0 + sw * (1.0 - fr) + jfr * 35.0 - 17.5, y0 + 0.178 * h + dy1, 5.0, 5.0);
        }
    })
}

fn circle(p: &egui::Painter, c: Rgbw, x: f64, y: f64, r: f64) {
    p.circle_filled(egui::Pos2::new(x as f32, y as f32), r as f32, color(c));
}

fn rect(p: &egui::Painter, c: Rgbw, x: f64, y: f64, w: f64, h: f64) {
    let rect = egui::Rect::from_center_size(egui::Pos2::new(x as f32, y as f32), egui::Vec2::new(w as f32, h as f32));
    p.rect_filled(rect, egui::Rounding::ZERO, color(c));
}

fn line(p: &egui::Painter, c: Rgbw, x0: f64, y0: f64, x1: f64, y1: f64) {}

fn color(c: impl Into<Rgb>) -> egui::Color32 {
    let Rgb(r, g, b) = c.into();
    egui::Color32::from_rgba_premultiplied(r.byte(), g.byte(), b.byte(), 255)
}

trait Map<T> {
    fn fmap<F: FnMut(usize, f64, f64, &mut T)>(&mut self, f: F);
    fn feach<F: FnMut(usize, f64, f64, &T)>(&self, f: F);
}

impl<T> Map<T> for [T] {
    fn fmap<F: FnMut(usize, f64, f64, &mut T)>(&mut self, mut f: F) {
        let n = self.len();
        for (i, t) in self.iter_mut().enumerate() {
            f(i, i as f64 / (n - 1) as f64, i as f64 / n as f64, t);
        }
    }
    fn feach<F: FnMut(usize, f64, f64, &T)>(&self, mut f: F) {
        let n = self.len();
        for (i, t) in self.iter().enumerate() {
            f(i, i as f64 / (n - 1) as f64, i as f64 / n as f64, t);
        }
    }
}
