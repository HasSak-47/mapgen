pub mod drawers;
pub mod general;
pub mod generator;
pub mod inputs;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use generator::wave::*;
use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect};

#[derive(Copy, Clone, Eq, PartialEq, Default)]
enum Tborder {
    Rig = 0b01,
    Lef = 0b10,
    Tot = 0b11,
    #[default]
    Non = 0b00,
}

impl Mirror for Tborder {
    fn mirror(&self) -> Self {
        match self {
            Tborder::Rig => Tborder::Lef,
            Tborder::Lef => Tborder::Rig,
            Tborder::Tot => Tborder::Tot,
            Tborder::Non => Tborder::Non,
        }
    }
}

const TILE_PIXELS: u32 = 32;
const TILE_CELLS: usize = 4;
const CELL_PIXELS: u32 = TILE_PIXELS / TILE_CELLS as u32;

#[derive(Clone, Copy)]
enum Palette {
    Sky,
    Grass,
    Dirt,
    Stone,
}

impl Palette {
    fn color(self) -> Color {
        match self {
            Palette::Sky => Color::RGB(126, 190, 214),
            Palette::Grass => Color::RGB(72, 151, 83),
            Palette::Dirt => Color::RGB(104, 86, 67),
            Palette::Stone => Color::RGB(86, 91, 94),
        }
    }
}

type TilePixels = [[Palette; TILE_CELLS]; TILE_CELLS];

fn tile_palette(tile: usize) -> TilePixels {
    use Palette::{Dirt as D, Grass as G, Sky as A, Stone as S};

    match tile {
        0 => [[A, A, A, A], [A, A, A, A], [A, A, A, A], [A, A, A, A]],
        1 => [[A, A, A, A], [A, A, A, A], [G, G, G, G], [D, D, D, D]],
        2 => [[S, S, S, S], [S, S, S, S], [S, S, S, S], [S, S, S, S]],
        3 => [[A, A, A, A], [A, A, A, A], [A, A, G, G], [A, A, D, D]],
        4 => [[A, A, S, S], [A, A, S, S], [A, A, S, S], [A, A, S, S]],
        5 => [[S, S, A, A], [S, S, A, A], [S, S, A, A], [S, S, A, A]],
        6 => [[A, A, A, A], [A, A, A, A], [G, G, A, A], [D, D, A, A]],
        7 => [[A, A, S, S], [A, A, S, S], [G, G, S, S], [D, D, S, S]],
        8 => [[S, S, A, A], [S, S, A, A], [S, S, G, G], [S, S, D, D]],

        _ => [[A, A, A, A], [A, A, A, A], [A, A, A, A], [A, A, A, A]],
    }
}

fn draw_tile(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    tile: usize,
    x: usize,
    y: usize,
) -> Result<(), String> {
    let px = x as i32 * TILE_PIXELS as i32;
    let py = y as i32 * TILE_PIXELS as i32;
    let pixels = tile_palette(tile);

    for row in 0..TILE_CELLS {
        for col in 0..TILE_CELLS {
            canvas.set_draw_color(pixels[row][col].color());
            canvas.fill_rect(Rect::new(
                px + col as i32 * CELL_PIXELS as i32,
                py + row as i32 * CELL_PIXELS as i32,
                CELL_PIXELS,
                CELL_PIXELS,
            ))?;
        }
    }

    Ok(())
}

fn draw_grid(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    width: usize,
    height: usize,
) -> Result<(), String> {
    let pixel_width = width as i32 * TILE_PIXELS as i32;
    let pixel_height = height as i32 * TILE_PIXELS as i32;

    canvas.set_draw_color(Color::RGB(220, 32, 32));
    for x in 0..=width {
        let px = x as i32 * TILE_PIXELS as i32;
        canvas.draw_line((px, 0), (px, pixel_height))?;
    }
    for y in 0..=height {
        let py = y as i32 * TILE_PIXELS as i32;
        canvas.draw_line((0, py), (pixel_width, py))?;
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let mut units = vec![
        Unit {
            // air
            north: Tborder::Non,
            south: Tborder::Non, //
            east: Tborder::Non,  //
            west: Tborder::Non,
        },
        Unit {
            // surface
            north: Tborder::Non,
            south: Tborder::Tot, // __
            east: Tborder::Rig,  // ##
            west: Tborder::Lef,
        },
        Unit {
            // ground
            north: Tborder::Tot,
            south: Tborder::Tot, // ##
            east: Tborder::Tot,  // ##
            west: Tborder::Tot,
        },
        Unit {
            //edge
            north: Tborder::Non,
            south: Tborder::Lef, //  -
            east: Tborder::Rig,  // |#
            west: Tborder::Non,
        },
    ];

    units.push(units[1].rotate(1));
    units.push(units[1].rotate(3));
    units.push(units[3].rotate(3));
    units.push(Unit {
        // left wall foot
        north: Tborder::Rig,
        south: Tborder::Tot,
        east: Tborder::Tot,
        west: Tborder::Non,
    });
    units.push(Unit {
        // right wall foot
        north: Tborder::Lef,
        south: Tborder::Tot,
        east: Tborder::Non,
        west: Tborder::Tot,
    });
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64;
    // let now = 1664905806579;

    let mut test_board = FiniteMap::<Tborder>::new(20, 20, units.clone(), now);
    test_board.determine();

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window_width = test_board.width as u32 * TILE_PIXELS;
    let window_height = test_board.height as u32 * TILE_PIXELS;
    let window = video
        .window("Wave Collapse Map", window_width, window_height)
        .position_centered()
        .build()
        .map_err(|err| err.to_string())?;
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|err| err.to_string())?;
    let mut events = sdl.event_pump()?;
    let mut show_grid = false;

    'running: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;
                    test_board = FiniteMap::<Tborder>::new(20, 20, units.clone(), now);
                    test_board.determine();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } => show_grid = !show_grid,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(126, 190, 214));
        canvas.clear();

        for screen_y in 0..test_board.height {
            let map_y = test_board.height - (screen_y + 1);
            for x in 0..test_board.width {
                let tile = test_board.map[x][map_y].collapse_val();
                draw_tile(&mut canvas, tile, x, screen_y)?;
            }
        }

        if show_grid {
            draw_grid(&mut canvas, test_board.width, test_board.height)?;
        }

        canvas.present();
    }

    Ok(())
}
