extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::{EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent, PressEvent, Button, ReleaseEvent, MouseButton, MouseCursorEvent};
use piston::window::WindowSettings;
use piston::input::keyboard::Key::{Space, Right, Left, Up, Down};
use graphics::math::Vec2d;
use vecmath;
use graphics::types;
use vecmath::{vec2_add, vec2_scale};
use std::ops::Index;
use std::f64;
use graphics::types::Color;

const TILE_SIZE: f64 = 50.0;

struct Player {
    pos: Vec2d<i32>,
    velocity: Vec2d<i32>,
}

impl Player {
    fn draw(&self, c: graphics::Context, gl: &mut GlGraphics) {
        use graphics::*;

        const PLAYER_TRIANGLE: types::Polygon = &[
            [0.0, 0.0],
            [30.0, 10.0],
            [0.0, 20.0],
        ];

        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

        let angle = (self.velocity[1] as f64).atan2(self.velocity[0] as f64);

        let transform = c
            .transform
            .trans(30.0, 25.0)
            .trans(self.pos[0] as f64 * TILE_SIZE, self.pos[1] as f64 * TILE_SIZE)
            .rot_rad(angle)
            .trans(-15.0, -10.0);

        polygon(RED, PLAYER_TRIANGLE, transform, gl)
    }

    fn forward(&mut self) {
        self.pos = vecmath::vec2_add(self.pos, self.velocity);
    }

    fn rotate_cw(&mut self) {
        self.velocity = match self.velocity {
            [0, -1] => [1, 0],
            [1, 0] => [0, 1],
            [0, 1] => [-1, 0],
            [-1, 0] => [0, -1],
            _ => panic!("invalid velocity [{}, {}]", self.velocity[0], self.velocity[1])
        }
    }
}

struct Tile {
    doors: [bool; 4],
    rotation: usize,
}

impl Tile {
    fn is_open(&self, velocity: Vec2d<i32>) -> bool {
        return match velocity {
            [0, -1] => self.doors[self.rotation%4],
            [1, 0] => self.doors[(self.rotation + 1)%4],
            [0, 1] => self.doors[(self.rotation + 2)%4],
            [-1, 0] => self.doors[(self.rotation + 3)%4],
            _ => panic!("invalid velocity [{}, {}]", velocity[0], velocity[1])
        }
    }

    fn rotate_cw(&mut self)  {
        self.rotation = self.rotation + 1
    }
}

struct World {
    player: Player,
    goal: Goal,
    map: [[Tile; 4]; 4],
}

impl World {
    fn draw(&self, c: graphics::Context, gl: &mut GlGraphics) {
        use graphics::*;

        for row in 0..self.map.len() {
            for col  in 0..self.map[0].len() {
                for side in 0..4 {
                    let color: Color;
                    if self.map[row][col].doors[side] {
                        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
                        color = GREEN
                    } else {
                        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
                        color = RED
                    }

                    let rot = (side + self.map[row][col].rotation) as f64 * f64::consts::PI/2.0;

//                let angle = (self.velocity[1] as f64).atan2(self.velocity[0] as f64);

                    let transform = c
                        .transform
                        .trans(25.0, 25.0)
                        .trans(col as f64 * TILE_SIZE, row as f64 * TILE_SIZE)
                        .rot_rad(rot)
                        .trans(-25.0, -25.0);

                    line_from_to(color, 1.0, [1.0, 1.0], [TILE_SIZE-1.0, 1.0],transform, gl)
                }
            }
        }

        self.player.draw(c, gl);
        self.goal.draw(c, gl);
    }

    fn tick(&mut self) {
        let curr_pos = self.player.pos;
        let next_pos = vec2_add(curr_pos, self.player.velocity);

        if next_pos[0] < 0 || next_pos[1] < 0 {
            return
        }

        if next_pos[0] as usize >= self.map.len() || next_pos[1] as usize >= self.map[0].len() {
            return
        }

        let curr_open = self.map[curr_pos[1] as usize][curr_pos[0] as usize].is_open(self.player.velocity);
        if !curr_open {
            return
        }

        let inv_velocity = vec2_scale(self.player.velocity, -1);
        let next_open = self.map[next_pos[1] as usize][next_pos[0] as usize].is_open(inv_velocity);

        if !next_open {
            return
        }

        self.player.forward()
    }
}

fn tile_index(pos: [f64; 2]) -> [usize; 2] {
    [(pos[0]/TILE_SIZE) as usize, (pos[1]/TILE_SIZE) as usize]
}

struct Goal {
    pos: Vec2d<i32>,
    rotation: f64,
}

impl Goal {
    fn draw(&self, c: graphics::Context, gl: &mut GlGraphics) {
        use graphics::*;

        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        let square = rectangle::square(0.0, 0.0, 30.0);

        let transform = c
            .transform
            .trans(30.0, 30.0)
            .trans(self.pos[0] as f64 * TILE_SIZE, self.pos[1] as f64 * TILE_SIZE)
            .rot_rad(self.rotation)
            .trans(-15.0, -15.0);

        // Draw a box rotating around the middle of the screen.
        rectangle(RED, square, transform, gl);
    }
}

pub struct App {
    gl: GlGraphics, // OpenGL drawing backend.
    world: World,
    mouse_pos: [f64; 2],
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const GREEN: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let world = &self.world;

        self.gl.draw(args.viewport(), |c, gl| {
            // Clear the screen.
            clear(GREEN, gl);

            world.draw(c, gl);
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        // Rotate 2 radians per second.
        self.world.goal.rotation += 2.0 * args.dt;
    }
}

const OPEN_TILE: Tile = Tile {
    doors: [true, true, true, true],
    rotation: 0,
};

const HORIZONTAL_TILE: Tile = Tile {
    doors: [false, true, false, true],
    rotation: 0,
};

const VERTCAL_TILE: Tile = Tile {
    doors: [true, false, true, false],
    rotation: 0,
};

const OPEN_WORLD: World = World {
    player: Player {
        pos: [0, 0],
        velocity: [1, 0],
    },
    goal: Goal {
        pos: [3, 3],
        rotation: 0.0,
    },
    map: [
        [OPEN_TILE, OPEN_TILE, OPEN_TILE, HORIZONTAL_TILE],
        [OPEN_TILE, VERTCAL_TILE, OPEN_TILE, OPEN_TILE],
        [OPEN_TILE, OPEN_TILE, VERTCAL_TILE, OPEN_TILE],
        [OPEN_TILE, OPEN_TILE, OPEN_TILE, OPEN_TILE]
    ],
};

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new("Straight Ahead!", [200, 200])
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        world: OPEN_WORLD,
        mouse_pos: [0.0, 0.0],
    };

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            app.render(&args);
        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            if key == Space {
                app.world.tick();
            }

            /*
            if key == Right {
                app.world.player.velocity = [1, 0]
            }
            if key == Left {
                app.world.player.velocity = [-1, 0]
            }
            if key == Up {
                app.world.player.velocity = [0, -1]
            }
            if key == Down {
                app.world.player.velocity = [0, 1]
            }
            */
        }

        if let Some(pos) = e.mouse_cursor_args() {
            app.mouse_pos = pos;
        };

        if let Some(button) = e.release_args() {
            if button == Button::Mouse(MouseButton::Left) {
                let idx = tile_index(app.mouse_pos);
                app.world.map[idx[1]][idx[0]].rotate_cw();

                if app.world.player.pos[0] == idx[0] as i32 && app.world.player.pos[1] == idx[1] as i32 {
                    app.world.player.rotate_cw();
                }
            }
        };

        if let Some(args) = e.update_args() {
            app.update(&args);
        }
    }
}
