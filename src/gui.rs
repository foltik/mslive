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
    let w = w0 * 0.9;
    let h = h0 * 0.9;
    let x0 = (w0 - w) * 0.5;
    let y0 = (h0 - h) * 0.5;

    // booth
    rect(p, Rgbw::BLACK, x0 + (w * 0.5), y0 + h, 350.0, 75.0);

    // rgbw bars
    {
        let bar_len = 100.0;
        let bar_w = 10.0;

        // back left
        rect(p, l.rgbw[0], x0 + (w * 0.30), y0 + (h * 0.50), bar_len, bar_w);
        // middle right
        rect(p, l.rgbw[1], x0 + (w * 0.80), y0 + (h * 0.40), bar_len, bar_w);
        // left wall
        rect(p, l.rgbw[2], x0 + (w * 0.00), y0 + (h * 0.60), bar_w, bar_len);
        // back right
        rect(p, l.rgbw[3], x0 + (w * 0.85), y0 + (h * 0.10), bar_len, bar_w);
        // front left
        rect(p, l.rgbw[4], x0 + (w * 0.30), y0 + (h * 0.65), bar_len, bar_w);
        // vertical front
        rect(p, l.rgbw[5], x0 + (w * 0.06), y0 + (h * 0.66), 4.0 * bar_w, 1.2 * bar_w);
        // vertical back
        rect(p, l.rgbw[6], x0 + (w * 0.06), y0 + (h * 0.59), 4.0 * bar_w, 1.2 * bar_w);
        // center
        rect(p, l.rgbw[7], x0 + (w * 0.5), y0 + (h * 0.50), bar_w, bar_len);
        // front right
        rect(p, l.rgbw[8], x0 + (w * 0.8), y0 + (h * 0.85), bar_len, bar_w);
    }

    // dimmers
    for x in 0..2 {
        for y in 0..4 {
            circle(
                p,
                Rgbw::WHITE * l.dimmer[(y * 2) + x],
                x0 + (w * 0.4) + (40.0 * y as f64),
                y0 + (h * 0.8) + (40.0 * x as f64),
                10.0,
            );
        }
    }
    // circle(p, l.pars[0].color, x0, y0 + dy, 10.0);
    // circle(p, l.pars[9].color, x0 + w, y0 + dy, 10.0);
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
