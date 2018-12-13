extern crate rand;
use rand::thread_rng;
use rand::Rng;

extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::*;
use piston::input::*;
use piston::window::WindowSettings;

pub struct App {
    gl: GlGraphics, // OpenGL drawing backend.
    maze: Maze,
}

// grid[cursor_y][cursor_x] = Cell::Cursor
// grid elements that are walls never change.
// Empty <=> Cursor <=> Visited only.
struct Maze {
    grid: Vec<Vec<Cell>>,
    height: usize,
    width: usize,
    cursor: (usize, usize),
    goal: (usize, usize),
}

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const DARK_GREEN: [f32; 4] = [0.0, 0.5, 0.0, 1.0];

impl Maze {
    pub fn generate_random(half_width: usize, half_height: usize) -> Self {
        let width = 2 * half_width - 1;
        let height = 2 * half_height - 1;
        let edges = {
            let mut possible_edges: Vec<((usize, usize), (usize, usize))> = Vec::new();
            let mut vertices_seen: Vec<(usize, usize)> = Vec::new();
            let mut out_edges: Vec<((usize, usize), (usize, usize))> = Vec::new();
            vertices_seen.push((0, 0));
            possible_edges.push(((0, 0), (1, 0)));
            possible_edges.push(((0, 0), (0, 1)));

            let mut rng = thread_rng();
            while !possible_edges.is_empty() {
                let new_edge_index = rng.gen_range(0, possible_edges.len());
                let new_edge = possible_edges.remove(new_edge_index);
                assert!(vertices_seen.contains(&new_edge.0));
                if !vertices_seen.contains(&new_edge.1) {
                    let old_vertex = new_edge.1;
                    if old_vertex.0 > 0 {
                        possible_edges.push((old_vertex, (old_vertex.0 - 1, old_vertex.1)));
                    }
                    if old_vertex.0 < half_height - 1 {
                        possible_edges.push((old_vertex, (old_vertex.0 + 1, old_vertex.1)));
                    }
                    if old_vertex.1 > 0 {
                        possible_edges.push((old_vertex, (old_vertex.0, old_vertex.1 - 1)));
                    }
                    if old_vertex.1 < half_width - 1 {
                        possible_edges.push((old_vertex, (old_vertex.0, old_vertex.1 + 1)));
                    }
                    vertices_seen.push(new_edge.1);
                    out_edges.push(new_edge);
                }
            }
            out_edges
        };
        let mut grid = vec![vec![Cell::Wall; width]; height];
        for ((start_row, start_col), (end_row, end_col)) in edges {
            let row = start_row + end_row;
            let col = start_col + end_col;
            grid[row][col] = Cell::Empty;
        }
        for row in 0..height / 2 + 1 {
            for col in 0..width / 2 + 1 {
                grid[2 * row][2 * col] = Cell::Empty;
            }
        }
        grid[0][0] = Cell::Cursor;
        grid[height-1][width-1] = Cell::Goal;
        Self {
            grid,
            height,
            width,
            cursor: (0, 0),
            goal: (height-1, width-1),
        }
    }

    fn move_delta(&mut self, dr: isize, dc: isize) {
        if self.cursor != self.goal {
            let (row, col) = self.cursor;
            let new_row = row as isize + dr;
            let new_col = col as isize + dc;
            if new_row >= 0 && new_col >= 0 {
                let new_row = new_row as usize;
                let new_col = new_col as usize;
                if new_row < self.height && new_col < self.width {
                    let old_target = &self.grid[new_row][new_col];
                    if *old_target != Cell::Wall {
                        self.grid[row][col] = old_target.flip();
                        self.grid[new_row][new_col] = Cell::Cursor;
                        self.cursor = (new_row, new_col);
                    }
                }
            }
        }
    }

    fn color_at_cell(&self, row: usize, col: usize) -> [f32; 4] {
        match self.grid[row][col] {
            Cell::Wall => BLACK,
            Cell::Empty => WHITE,
            Cell::Visited => RED,
            Cell::Cursor => GREY,
            Cell::Goal => DARK_GREEN,
        }
    }

    fn rectangle_at_cell(
        &self,
        window_width: f64,
        window_height: f64,
        row: usize,
        col: usize,
    ) -> graphics::types::Rectangle {
        use graphics::*;
        let border = if window_width / (self.width as f64) < 4.0
            || window_height / (self.height as f64) < 4.0
        {
            1.0
        } else {
            2.0
        };
        let box_width = (window_width - (self.width + 1) as f64 * border) / self.width as f64;
        let box_height = (window_height - (self.height + 1) as f64 * border) / self.height as f64;

        let left_x = (border + box_width) * col as f64 + border;
        let right_x = (border + box_width) * (col + 1) as f64;
        let top_y = (border + box_height) * row as f64 + border;
        let bottom_y = (border + box_height) * (row + 1) as f64;

        rectangle::rectangle_by_corners(left_x, top_y, right_x, bottom_y)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Cell {
    Wall,
    Empty,
    Visited,
    Cursor,
    Goal,
}

impl Cell {
    fn flip(&self) -> Self {
        match self {
            Cell::Wall => panic!("Wall cannot be flipped"),
            Cell::Cursor => panic!("Cursor cannot be flipped"),
            Cell::Empty => Cell::Visited,
            Cell::Visited => Cell::Empty,
            Cell::Goal => Cell::Visited,
        }
    }
}

impl App {
    // Rendering from scratch
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let maze = &self.maze;

        self.gl.draw(args.viewport(), |c, gl| {
            clear(BLACK, gl);

            for row in 0..maze.height {
                for col in 0..maze.width {
                    let color = maze.color_at_cell(row, col);
                    let box_rect = maze.rectangle_at_cell(args.width, args.height, row, col);
                    rectangle(color, box_rect, c.transform, gl);
                }
            }
        });
    }

    fn update_button(&mut self, args: &ButtonArgs) {
        if let ButtonState::Press = args.state {
            match args.button {
                Button::Keyboard(keyboard::Key::Up) => self.maze.move_delta(-1, 0),
                Button::Keyboard(keyboard::Key::Down) => self.maze.move_delta(1, 0),
                Button::Keyboard(keyboard::Key::Left) => self.maze.move_delta(0, -1),
                Button::Keyboard(keyboard::Key::Right) => self.maze.move_delta(0, 1),
                _ => (),
            }
        }
    }
}
fn main() {
    let mut args = std::env::args();
    args.next();
    let width = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);
    let height = args.next().and_then(|s| s.parse().ok()).unwrap_or(width);
    let opengl = OpenGL::V3_2;

    let mut window: Window = WindowSettings::new("maze", [800, 600])
        .opengl(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        maze: Maze::generate_random(width, height),
    };

    let mut events = Events::new(EventSettings::new());

    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r);
        }
        if let Some(b) = e.button_args() {
            app.update_button(&b);
        }
    }
}
