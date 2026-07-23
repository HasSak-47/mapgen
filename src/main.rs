pub mod drawers;
pub mod general;
pub mod generator;
pub mod inputs;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use generator::wave::*;
use sdl2::{event::Event, keyboard::Keycode, mouse::MouseButton, pixels::Color, rect::Rect};

#[derive(Copy, Debug, Clone, Eq, PartialEq, Default)]
enum Tile {
    #[default]
    Sky,
    Floor,
    Stone,
    RightEdge,
    LeftEdge,
    RightCliff,
    LeftCliff,
    RightFloorCliff,
    LeftFloorCliff,
}

impl Tile {
    fn can_connect_horizontal(left: Self, right: Self) -> bool {
        use Tile::{
            Floor, LeftCliff as LCliff, LeftEdge as LEdge, LeftFloorCliff as LFCliff,
            RightCliff as RCliff, RightEdge as REdge, RightFloorCliff as RFCliff, Sky, Stone,
        };

        match (left, right) {
            (Sky, Sky | REdge | RCliff) => true,
            (LEdge | LCliff, Sky) => true,
            (Floor | REdge | LFCliff, Floor | LEdge | RFCliff) => true,
            (RCliff, Stone | LCliff | LFCliff) => true,
            (Stone | RFCliff, Stone | LCliff | LFCliff) => true,
            _ => false,
        }
    }

    fn can_connect_vertical(lower: Self, upper: Self) -> bool {
        use Tile::{
            Floor, LeftCliff as LCliff, LeftEdge as LEdge, LeftFloorCliff as LFCliff,
            RightCliff as RCliff, RightEdge as REdge, RightFloorCliff as RFCliff, Sky, Stone,
        };

        match (lower, upper) {
            (Sky, Sky) => true,
            (Floor | REdge | LEdge, Sky) => true,
            (Stone, Stone | Floor | RFCliff | LFCliff) => true,
            (RCliff | RFCliff, RCliff | REdge) => true,
            (LCliff | LFCliff, LCliff | LEdge) => true,
            _ => false,
        }
    }
}

impl TileConnection for Tile {
    fn can_connect(&self, _x: usize, _y: usize, direction: Direction, other: &Self) -> u64 {
        if match direction {
            Direction::East => Tile::can_connect_horizontal(*self, *other),
            Direction::West => Tile::can_connect_horizontal(*other, *self),
            Direction::North => Tile::can_connect_vertical(*self, *other),
            Direction::South => Tile::can_connect_vertical(*other, *self),
        } {
            1
        } else {
            0
        }
    }
}

const TILE_PIXELS: u32 = 32;
const SPRITE_CELLS: usize = 4;
const CONTRADICTION_BORDER: u32 = 3;

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

type Sprite = [[Palette; SPRITE_CELLS]; SPRITE_CELLS];

fn tile_sprite(tile: Tile) -> Sprite {
    use Palette::{Dirt as D, Grass as G, Sky as A, Stone as S};

    match tile {
        Tile::Sky => [[A, A, A, A], [A, A, A, A], [A, A, A, A], [A, A, A, A]],
        Tile::Floor => [[G, G, G, G], [D, D, D, D], [D, D, D, D], [D, D, D, D]],
        Tile::Stone => [[S, S, S, S], [S, S, S, S], [S, S, S, S], [S, S, S, S]],
        Tile::RightEdge => [[A, A, G, G], [A, A, D, D], [A, A, D, D], [A, A, D, D]],
        Tile::LeftEdge => [[G, G, A, A], [D, D, A, A], [D, D, A, A], [D, D, A, A]],
        Tile::RightCliff => [[A, A, S, S], [A, A, S, S], [A, A, S, S], [A, A, S, S]],
        Tile::LeftCliff => [[S, S, A, A], [S, S, A, A], [S, S, A, A], [S, S, A, A]],
        Tile::RightFloorCliff => [[G, G, S, S], [D, D, S, S], [D, D, S, S], [D, D, S, S]],
        Tile::LeftFloorCliff => [[S, S, G, G], [S, S, D, D], [S, S, D, D], [S, S, D, D]],
    }
}

fn draw_sprite(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    sprite: Sprite,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    for row in 0..SPRITE_CELLS {
        for col in 0..SPRITE_CELLS {
            let x0 = col as u32 * width / SPRITE_CELLS as u32;
            let y0 = row as u32 * height / SPRITE_CELLS as u32;
            let x1 = (col as u32 + 1) * width / SPRITE_CELLS as u32;
            let y1 = (row as u32 + 1) * height / SPRITE_CELLS as u32;

            canvas.set_draw_color(sprite[row][col].color());
            canvas.fill_rect(Rect::new(x + x0 as i32, y + y0 as i32, x1 - x0, y1 - y0))?;
        }
    }

    Ok(())
}

fn draw_state(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    possible: &[Tile],
    x: usize,
    y: usize,
) -> Result<(), String> {
    let px = x as i32 * TILE_PIXELS as i32;
    let py = y as i32 * TILE_PIXELS as i32;

    if possible.is_empty() {
        canvas.set_draw_color(Color::RGB(24, 24, 24));
        canvas.fill_rect(Rect::new(px, py, TILE_PIXELS, TILE_PIXELS))?;
        canvas.set_draw_color(Color::RGB(255, 72, 190));
        for inset in 0..CONTRADICTION_BORDER {
            let inset_i = inset as i32;
            let size = TILE_PIXELS.saturating_sub(inset * 2);
            canvas.draw_rect(Rect::new(px + inset_i, py + inset_i, size, size))?;
        }
        return Ok(());
    }

    if possible.len() == 1 {
        draw_sprite(
            canvas,
            tile_sprite(possible[0]),
            px,
            py,
            TILE_PIXELS,
            TILE_PIXELS,
        )?;
        return Ok(());
    }

    let cols = (possible.len() as f64).sqrt().ceil() as usize;
    let rows = (possible.len() + cols - 1) / cols;
    let gap = 1;

    canvas.set_draw_color(Color::RGB(24, 24, 24));
    canvas.fill_rect(Rect::new(px, py, TILE_PIXELS, TILE_PIXELS))?;

    for (i, tile) in possible.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x0 = col as u32 * TILE_PIXELS / cols as u32;
        let y0 = row as u32 * TILE_PIXELS / rows as u32;
        let x1 = (col as u32 + 1) * TILE_PIXELS / cols as u32;
        let y1 = (row as u32 + 1) * TILE_PIXELS / rows as u32;
        let width = x1 - x0;
        let height = y1 - y0;

        draw_sprite(
            canvas,
            tile_sprite(*tile),
            px + x0 as i32 + gap,
            py + y0 as i32 + gap,
            width.saturating_sub(gap as u32 * 2),
            height.saturating_sub(gap as u32 * 2),
        )?;
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
    let tiles = vec![
        Tile::Sky,
        Tile::Floor,
        Tile::Stone,
        Tile::RightEdge,
        Tile::LeftEdge,
        Tile::RightCliff,
        Tile::LeftCliff,
        Tile::RightFloorCliff,
        Tile::LeftFloorCliff,
    ];

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64;
    // let now = 1664905806579;

    let mut test_board = FiniteMap::<Tile>::new(20, 20, tiles.clone(), now);

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
    let mut random_fallback = false;

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
                    test_board = FiniteMap::<Tile>::new(20, 20, tiles.clone(), now);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::N),
                    repeat: false,
                    ..
                } => {
                    if random_fallback {
                        test_board.step_with_random_fallback();
                    } else {
                        test_board.step();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::M),
                    repeat: false,
                    ..
                } => {
                    if random_fallback {
                        test_board.substep_with_random_fallback();
                    } else {
                        test_board.substep();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F),
                    repeat: false,
                    ..
                } => random_fallback = !random_fallback,
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    repeat: false,
                    ..
                } => while test_board.step_with_random_fallback() {},
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } => show_grid = !show_grid,
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                } if x >= 0 && y >= 0 => {
                    let tile_x = x as usize / TILE_PIXELS as usize;
                    let screen_tile_y = y as usize / TILE_PIXELS as usize;

                    if tile_x < test_board.width && screen_tile_y < test_board.height {
                        let tile_y = test_board.height - (screen_tile_y + 1);

                        if test_board.map[tile_x][tile_y].entropy() > 0 {
                            test_board.force_collapse(tile_x, tile_y);
                        }
                    }
                }
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(126, 190, 214));
        canvas.clear();

        for screen_y in 0..test_board.height {
            let map_y = test_board.height - (screen_y + 1);
            for x in 0..test_board.width {
                let possible = match &test_board.map[x][map_y] {
                    Cell::Collapsed(tile) => vec![test_board.possible[*tile]],
                    Cell::Uncollapsed(tiles) => tiles
                        .iter()
                        .map(|tile| test_board.possible[*tile])
                        .collect(),
                };
                draw_state(&mut canvas, &possible, x, screen_y)?;
            }
        }

        if show_grid {
            draw_grid(&mut canvas, test_board.width, test_board.height)?;
        }

        canvas.present();
    }

    Ok(())
}
