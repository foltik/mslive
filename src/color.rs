use stagebridge::num::Float;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color {
    pub a: f64,
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub w: f64
}

impl Color {
    pub const OFF:     Self = Self::argbw(0.0, 0.0, 0.0, 0.0, 0.0);
    pub const WHITE:   Self = Self::w(1.0);
    pub const RGB:     Self = Self::rgb(1.0,   1.0,   1.0);

    pub const RED:     Self = Self::rgb(1.0,   0.0,   0.0);
    pub const ORANGE:  Self = Self::rgb(1.0,   0.251, 0.0);
    pub const YELLOW:  Self = Self::rgb(1.0,   1.0,   0.0);
    pub const PEA:     Self = Self::rgb(0.533, 1.0,   0.0);
    pub const LIME:    Self = Self::rgb(0.0,   1.0,   0.0);
    pub const MINT:    Self = Self::rgb(0.0,   1.0,   0.267);
    pub const CYAN:    Self = Self::rgb(0.0,   0.8,   1.0);
    pub const BLUE:    Self = Self::rgb(0.0,   0.0,   1.0);
    pub const VIOLET:  Self = Self::rgb(0.533, 0.0,   1.0);
    pub const MAGENTA: Self = Self::rgb(1.0,   0.0,   1.0);
    pub const PINK:    Self = Self::rgb(1.0,   0.38,  0.8);

    pub const fn argbw(a: f64, r: f64, g: f64, b: f64, w: f64) -> Self { Self { a, r, g, b, w } }
    pub const fn argb(a: f64, r: f64, g: f64, b: f64) -> Self { Self::argbw(a, r, g, b, 0.0) }
    pub const fn aw(a: f64, w: f64) -> Self { Self::argbw(a, 0.0, 0.0, 0.0, w) }
    pub const fn rgbw(r: f64, g: f64, b: f64, w: f64) -> Self { Self::argbw(1.0, r, g, b, w) }
    pub const fn rgb(r: f64, g: f64, b: f64) -> Self { Self::argb(1.0, r, g, b) }
    pub const fn w(w: f64) -> Self { Self::aw(1.0, w) }

    pub fn hsv(h: f64, s: f64, v: f64) -> Self {
        let r = v * s.lerp(1.0..(((h + 1.0      ).fract() * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0));
        let g = v * s.lerp(1.0..(((h + 0.6666666).fract() * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0));
        let b = v * s.lerp(1.0..(((h + 0.3333333).fract() * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0));
        Self::rgb(r, g, b)
    }
}

impl Color {
    pub fn a(self, a: f64) -> Self {
        Self { a, r: self.r, g: self.g, b: self.b, w: self.w }
    }

    pub fn a_mul(self, a: f64) -> Self {
        Self { a: self.a * a, r: self.r, g: self.g, b: self.b, w: self.w }
    }
}

use stagebridge::midi::device::launchpad_x::types::PaletteColor;
impl From<PaletteColor> for Color {
    fn from(p: PaletteColor) -> Self {
        match p {
            PaletteColor::Index(_)   => Color::WHITE,
            PaletteColor::Off        => Color::OFF,
            PaletteColor::White      => Color::WHITE,
            PaletteColor::Red        => Color::RED,
            PaletteColor::Orange     => Color::ORANGE,
            PaletteColor::Yellow     => Color::YELLOW,
            PaletteColor::Pea        => Color::PEA,
            PaletteColor::Lime       => Color::LIME,
            PaletteColor::Mint       => Color::MINT,
            PaletteColor::Cyan       => Color::CYAN,
            PaletteColor::Blue       => Color::BLUE,
            PaletteColor::Violet     => Color::VIOLET,
            PaletteColor::Magenta    => Color::MAGENTA,
            PaletteColor::Pink       => Color::PINK,
        }
    }
}

pub use stagebridge::midi::device::launchpad_x::types::Color as PadColor;
impl From<Color> for PadColor {
    fn from(color: Color) -> Self {
        if color == Color::WHITE || color == Color::RGB {
            PadColor::Palette(PaletteColor::White)
        } else {
            let Color { r, g, b, a, .. } = color;
            PadColor::Rgb(r * a, g * a, b * a)
        }
    }
}
