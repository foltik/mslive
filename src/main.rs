use std::{time::Duration, collections::VecDeque, thread};
use color_eyre::Result;
use rand::Rng;

use stagebridge::midi::{Midi, device::launchpad_x::{*, types::*}};

enum Direction {
    Left,
    Right,
    Up,
    Down,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    pretty_env_logger::init();

    let mut rng = rand::thread_rng();
    let mut rand_pos = move || Coord(rng.gen_range(0..8), rng.gen_range(0..8));

    let mut pad = Midi::connect("Launchpad X:Launchpad X LPX MIDI", LaunchpadX::default())?;
    pad.send(Output::Mode(Mode::Programmer));
    pad.send(Output::Pressure(Pressure::Off, PressureCurve::Medium));
    pad.send(Output::Clear);

    // background color
    pad.send(Output::ClearColor(Color::Palette(PaletteColor::Index(1))));
    // arrow keys color
    pad.send(Output::Light(Coord(0, 8).into(), PaletteColor::Index(41)));
    pad.send(Output::Light(Coord(1, 8).into(), PaletteColor::Index(41)));
    pad.send(Output::Light(Coord(2, 8).into(), PaletteColor::Index(41)));
    pad.send(Output::Light(Coord(3, 8).into(), PaletteColor::Index(41)));

    let mut dir = Direction::Right;
    let mut snake = VecDeque::from_iter([Coord(0, 0)]);
    let mut fruit = rand_pos();

    // initial fruit color
    pad.send(Output::Light(fruit.into(), PaletteColor::Index(5)));

    loop {
        for input in pad.recv() {
            match input {
                Input::Up(true) => { dir = Direction::Up; },
                Input::Down(true) => { dir = Direction::Down; },
                Input::Left(true) => { dir = Direction::Left; },
                Input::Right(true) => { dir = Direction::Right; },
                _ => {}
            }
        }

        let Coord(mut x, mut y) = snake.front().unwrap();
        let (dx, dy) = match dir {
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
            Direction::Up => (0, 1),
            Direction::Down => (0, -1),
        };

        // wrap X
        x += dx;
        if x > 7 {
            x = 0;
        }
        if x < 0 {
            x = 7;
        }

        // wrap Y
        y += dy;
        if y > 7 {
            y = 0;
        }
        if y < 0 {
            y = 7;
        }

        // push new head
        let head = Coord(x, y);
        snake.push_front(head);
        pad.send(Output::Light(head.into(), PaletteColor::Index(21)));

        if head == fruit {
            // if we ate a fruit, move it
            fruit = rand_pos();
            let mut good = true;
            while !good {
                for c in &snake {
                    good = good && fruit != *c;
                }

                if !good {
                    fruit = rand_pos();
                }
            }
            pad.send(Output::Light(fruit.into(), PaletteColor::Index(5)));
        } else {
            // otherwise, move the tail forward
            let tail = snake.pop_back().unwrap();
            pad.send(Output::Light(tail.into(), PaletteColor::Index(1)));
        }

        thread::sleep(Duration::from_millis(500));
    }
}
