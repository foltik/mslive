use stagebridge::dmx::Device;
use stagebridge::num::Interp;

#[derive(Clone, Copy, Debug, Default)]
pub struct Scanner2 {
    pub on: bool,
    pub pitch: f64,
    pub yaw: f64,
    pub color: f64,
    pub gobo: f64,
}

impl Device for Scanner2 {
    fn channels(&self) -> usize {
        7
    }

    fn encode(&self, buf: &mut [u8]) {
        buf[0] = 0;
        buf[1] = self.yaw.byte();
        buf[2] = self.pitch.byte();
        buf[3] = 20;
        buf[4] = self.color.byte();
        buf[5] = self.gobo.byte();
        buf[5] = self.gobo.clamp(0.0, 1.0).lerp(0..167) as u8;
        buf[6] = if self.on { 255 } else { 0 };
    }
}
