use stagebridge::dmx::Device;
use stagebridge::num::Interp;

#[derive(Clone, Copy, Debug, Default)]
pub struct Scanner1 {
    pub on: bool,
    pub brightness: f64,
    pub gobo: f64,

    pub rotate: f64,
    pub spread: f64,
    pub scroll: f64,
}

impl Device for Scanner1 {
    fn channels(&self) -> usize {
        5
    }

    fn encode(&self, buf: &mut [u8]) {
        // buf[0] = if self.on { 167 } else { 0 };
        // buf[0] = self.brightness.clamp(0.0, 1.0).lerp(5..154) as u8;
        buf[0] = self.brightness.byte();
        buf[1] = self.rotate.clamp(0.0, 1.0).lerp(0..106) as u8;
        buf[2] = self.gobo.clamp(0.0, 1.0).lerp(0..170) as u8;
        buf[3] = self.spread.byte();
        buf[4] = self.scroll.clamp(0.0, 1.0).lerp(0..120) as u8;
    }
}
