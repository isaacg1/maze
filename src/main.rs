use std::collections::HashMap;
use std::collections::HashSet;

extern crate rand;
use rand::thread_rng;
use rand::Rng;

extern crate noisy_float;
use noisy_float::prelude::*;

extern crate find_folder;

extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
extern crate piston_window;

use graphics::character::CharacterCache;
use opengl_graphics::OpenGL;
use piston::event_loop::*;
use piston::input::*;
use piston_window::{Glyphs, PistonWindow, TextureSettings, WindowSettings};

struct VecSet<T> {
    set: HashSet<T>,
    vec: Vec<T>,
}

impl<T> VecSet<T>
where
    T: Clone + Eq + std::hash::Hash,
{
    fn new() -> Self {
        Self {
            set: HashSet::new(),
            vec: Vec::new(),
        }
    }
    fn insert(&mut self, elem: T) {
        assert_eq!(self.set.len(), self.vec.len());
        let was_new = self.set.insert(elem.clone());
        if was_new {
            self.vec.push(elem);
        }
    }
    fn remove_random(&mut self) -> T {
        assert_eq!(self.set.len(), self.vec.len());
        let index = thread_rng().gen_range(0, self.vec.len());
        let elem = self.vec.swap_remove(index);
        let was_present = self.set.remove(&elem);
        assert!(was_present);
        elem
    }
    fn is_empty(&self) -> bool {
        assert_eq!(self.set.len(), self.vec.len());
        self.vec.is_empty()
    }
}

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const DARK_GREEN: [f32; 4] = [0.0, 0.5, 0.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const DARK_PURPLE: [f32; 4] = [0.5, 0.0, 0.5, 1.0];

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

impl Maze {
    pub fn generate_random(half_width: usize, half_height: usize) -> Self {
        let width = 2 * half_width - 1;
        let height = 2 * half_height - 1;
        let edges = {
            let mut possible_edges: VecSet<((usize, usize), (usize, usize))> = VecSet::new();
            let mut vertices_seen: HashMap<(usize, usize), bool> = HashMap::new();
            let mut out_edges: Vec<((usize, usize), (usize, usize))> = Vec::new();
            vertices_seen.insert((0, 0), true);
            vertices_seen.insert((half_height - 1, half_width - 1), false);
            possible_edges.insert(((0, 0), (1, 0)));
            possible_edges.insert(((0, 0), (0, 1)));
            possible_edges.insert((
                (half_height - 1, half_width - 1),
                (half_height - 2, half_width - 1),
            ));
            possible_edges.insert((
                (half_height - 1, half_width - 1),
                (half_height - 1, half_width - 2),
            ));
            let mut colors_crossed = false;

            while !possible_edges.is_empty() {
                let new_edge = possible_edges.remove_random();
                let old_color = vertices_seen.get(&new_edge.0).expect("Old vertex was seen");
                let previous_new_color = vertices_seen.get(&new_edge.1);
                if previous_new_color.is_none()
                    || !colors_crossed && previous_new_color == Some(&!old_color)
                {
                    let old_vertex = new_edge.1;
                    if old_vertex.0 > 0 {
                        possible_edges.insert((old_vertex, (old_vertex.0 - 1, old_vertex.1)));
                    }
                    if old_vertex.0 < half_height - 1 {
                        possible_edges.insert((old_vertex, (old_vertex.0 + 1, old_vertex.1)));
                    }
                    if old_vertex.1 > 0 {
                        possible_edges.insert((old_vertex, (old_vertex.0, old_vertex.1 - 1)));
                    }
                    if old_vertex.1 < half_width - 1 {
                        possible_edges.insert((old_vertex, (old_vertex.0, old_vertex.1 + 1)));
                    }
                    if previous_new_color.is_none() {
                        vertices_seen.insert(new_edge.1, *old_color);
                    } else {
                        colors_crossed = true;
                    }
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
        grid[height - 1][width - 1] = Cell::Goal;
        Self {
            grid,
            height,
            width,
            cursor: (0, 0),
            goal: (height - 1, width - 1),
        }
    }

    fn move_delta(&mut self, dr: isize, dc: isize) {
        if !self.is_done() {
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

    fn is_done(&self) -> bool {
        self.cursor == self.goal
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

pub struct App {
    window: PistonWindow,
    maze: Maze,
    past_completions: Vec<f64>,
    time: f64,
    completion_time: Option<f64>,
    glyphs: Glyphs,
}

impl App {
    // Rendering from scratch
    fn render(&mut self, args: &RenderArgs, event: &Event) {
        use graphics::*;

        let maze = &self.maze;

        let time_str = format!("{:.1}s", self.completion_time.unwrap_or(self.time));
        let text_color = if self.completion_time.is_some() {
            GREEN
        } else {
            WHITE
        };
        let past_str = if self.past_completions.is_empty() {
            None
        } else {
            let max_past = self
                .past_completions
                .iter()
                .max_by_key(|&&f| n64(f))
                .unwrap();
            let min_past = self
                .past_completions
                .iter()
                .min_by_key(|&&f| n64(f))
                .unwrap();
            let avg_past =
                self.past_completions.iter().sum::<f64>() / self.past_completions.len() as f64;
            Some(format!(
                "{:.1}s  {:.1}s  {:.1}s",
                min_past, avg_past, max_past
            ))
        };

        let glyphs = &mut self.glyphs;
        self.window.draw_2d(event, |c, gl| {
            clear(BLACK, gl);

            let top_box = rectangle::rectangle_by_corners(0.0, 0.0, args.width, args.height * 0.2);
            rectangle(DARK_PURPLE, top_box, c.transform, gl);

            let text_width = glyphs.width(36, &time_str).expect("Successful text width");
            let text_transform;
            if let Some(past_str) = past_str {
                let past_width = glyphs.width(36, &past_str).expect("Successful past width");;
                let past_transform = c
                    .transform
                    .trans(args.width / 3.0 - past_width / 2.0, args.height * 0.2 / 2.0);
                text::Text::new_color(WHITE, 36)
                    .round()
                    .draw(&past_str, glyphs, &c.draw_state, past_transform, gl)
                    .expect("Successful past drawing");
                text_transform = c.transform.trans(
                    args.width * 2.0 / 3.0 - text_width / 2.0,
                    args.height * 0.2 / 2.0,
                );
            } else {
                text_transform = c
                    .transform
                    .trans(args.width / 2.0 - text_width / 2.0, args.height * 0.2 / 2.0);
            }
            text::Text::new_color(text_color, 36)
                .round()
                .draw(&time_str, glyphs, &c.draw_state, text_transform, gl)
                .expect("Successful text drawing");

            for row in 0..maze.height {
                for col in 0..maze.width {
                    let color = maze.color_at_cell(row, col);
                    let box_rect = maze.rectangle_at_cell(args.width, args.height, row, col);
                    let transform = c
                        .transform
                        .scale(1.0, 0.8)
                        .trans(0.0, args.height * 0.2 / 0.8);
                    rectangle(color, box_rect, transform, gl);
                }
            }
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.time += args.dt;
    }

    fn update_button(&mut self, args: &ButtonArgs) {
        if let ButtonState::Press = args.state {
            match args.button {
                Button::Keyboard(keyboard::Key::Up) => self.maze.move_delta(-1, 0),
                Button::Keyboard(keyboard::Key::Down) => self.maze.move_delta(1, 0),
                Button::Keyboard(keyboard::Key::Left) => self.maze.move_delta(0, -1),
                Button::Keyboard(keyboard::Key::Right) => self.maze.move_delta(0, 1),
                Button::Keyboard(keyboard::Key::R) => self.reset(),
                _ => (),
            }
            if self.maze.is_done() && self.completion_time.is_none() {
                self.completion_time = Some(self.time);
            }
        }
    }

    fn reset(&mut self) {
        if let Some(completion_time) = self.completion_time {
            self.past_completions.push(completion_time);
        }
        let old_width = self.maze.width;
        let old_height = self.maze.height;
        self.maze = Maze::generate_random(old_width / 2 + 1, old_height / 2 + 1);
        assert_eq!(old_width, self.maze.width);
        assert_eq!(old_height, self.maze.height);
        self.time = 0.0;
        self.completion_time = None;
    }
}
fn main() {
    let mut args = std::env::args();
    args.next();
    let width = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);
    let height = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(width * 3 / 5);
    let opengl = OpenGL::V3_2;

    let window: PistonWindow = WindowSettings::new("maze", [800, 600])
        .opengl(opengl)
        .exit_on_esc(true)
        .fullscreen(true)
        .build()
        .unwrap();

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .expect("An assets folder");
    let font = assets.join("FiraSans-Regular.ttf");
    let factory = window.factory.clone();
    let glyphs = Glyphs::new(&font, factory, TextureSettings::new()).expect("Got glyphs");

    // Create a new game and run it.
    let mut app = App {
        window: window,
        maze: Maze::generate_random(width, height),
        time: 0.0,
        completion_time: None,
        past_completions: vec![],
        glyphs,
    };

    let mut events = Events::new(EventSettings::new());

    while let Some(e) = events.next(&mut app.window) {
        if let Some(r) = e.render_args() {
            app.render(&r, &e);
        }
        if let Some(b) = e.button_args() {
            app.update_button(&b);
        }
        if let Some(u) = e.update_args() {
            app.update(&u);
        }
    }
}
