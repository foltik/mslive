use stagebridge::color::Rgbw;
use stagebridge::dmx::Device;
use stagebridge::num::Interp;

#[derive(Clone, Copy, Debug, Default)]
pub struct Bar {
    pub angle: f64,
    pub beads: [Rgbw; 10],
}

impl Device for Bar {
    fn channels(&self) -> usize {
        42
    }

    fn encode(&self, buf: &mut [u8]) {
        buf[0] = self.angle.byte();
        buf[1] = 0;
        for (i, rgbw) in self.beads.iter().enumerate() {
            buf[2 + 0 + (i * 4)] = rgbw.0.byte();
            buf[2 + 1 + (i * 4)] = rgbw.1.byte();
            buf[2 + 2 + (i * 4)] = rgbw.2.byte();
            buf[2 + 3 + (i * 4)] = rgbw.3.byte();
        }
    }
}
