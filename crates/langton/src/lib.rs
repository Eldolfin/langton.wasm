use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugColor, DebugUI, Param};
use engine::Simulation;
use engine_macros::SimulationConfig;
#[derive(SimulationConfig)]
pub struct GameConfig {
    #[param(
        section = "Canvas",
        name = "start x",
        default = "0.80",
        range = "0.0..=1.0",
        step = 0.01,
        needs_restart
    )]
    pub start_x_rel: Param<f32>,
    #[param(
        name = "start y",
        default = "0.75",
        range = "0.0..=1.0",
        step = 0.01,
        needs_restart
    )]
    pub start_y_rel: Param<f32>,
    #[param(
        section = "Ants",
        name = "number of ants",
        default = "2",
        range = "1..=1000",
        scale = "Logarithmic"
    )]
    pub num_ants: Param<usize>,
    #[param(
        name = "ant color saturation",
        default = "0.3",
        range = "0.0..=1.0",
        step = 0.01
    )]
    pub ant_color_saturation: Param<f32>,
    #[param(
        name = "ant color brightness",
        default = "0.7",
        range = "0.0..=1.0",
        step = 0.01
    )]
    pub ant_color_brightness: Param<f32>,
    #[param(
        section = "Visual",
        name = "cell size",
        default = "20",
        range = "1..=50"
    )]
    pub cell_size: Param<usize>,
    #[param(name = "cell border size", default = "1", range = "0..=5")]
    pub cell_border_size: Param<usize>,
    #[param(
        name = "common cell color",
        default = "DebugColor { r: 30, g: 30, b: 30 }",
        color
    )]
    pub common_cell_color: Param<DebugColor>,
    #[param(
        section = "Advanced",
        name = "seed",
        default = "0",
        range = "0..=4294967295",
        needs_restart
    )]
    pub seed: Param<u32>,
}

pub const LANGTON_PRESETS: &[(&str, &str)] = &[
    (
        "Many small ants",
        "alpha_retention=235&cell_size=5&final_speed=0.5&number_of_ants=400&speedup_frames=0&start_x=0.5&start_y=0.5",
    ),
    (
        "3 trailing ants",
        "alpha_retention=255&final_speed=30&number_of_ants=3&speedup_frames=300&start_x=0.5&start_y=0.5&cell_size=4",
    ),
    (
        "Angry ant",
        "alpha_retention=220&final_speed=200&number_of_ants=1&speedup_frames=0",
    ),
    (
        "Flies",
        "alpha_retention=0&ant_color_brightness=0.3&ant_color_saturation=0&cell_border_size=0&cell_size=6&final_speed=1&number_of_ants=500&speedup_frames=120&start_x=0.5&start_y=0.5&common_cell_color=%23000000",
    ),
    (
        "Chaos",
        "alpha_retention=255&final_speed=40&number_of_ants=300&speedup_frames=600&start_x=0.5&start_y=0.5",
    ),
    (
        "Small grid",
        "alpha_retention=254&ant_color_brightness=0.65&ant_color_saturation=1&cell_border_size=0&cell_size=5&final_speed=25&number_of_ants=4&speedup_frames=1200&start_x=0.5&start_y=0.5&common_cell_color=%23E3E3E3",
    ),
    (
        "1px grid",
        "alpha_retention=255&ant_color_brightness=0&ant_color_saturation=0.5&cell_border_size=0&cell_size=1&final_speed=5000&number_of_ants=1&speedup_frames=0&common_cell_color=%23FFFFFF",
    ),
    (
        "Github",
        "alpha_retention=255&cell_border_size=0&cell_size=4&common_cell_color=%230D1117&debug=&final_speed=90&number_of_ants=3&speedup_frames=1222&start_x=0.5&start_y=0.5",
    ),
];

#[derive(Debug, Clone, Copy, Default)]
enum Direction {
    #[default]
    North,
    Est,
    South,
    West,
}

impl Direction {
    fn left(self) -> Self {
        match self {
            Direction::North => Direction::West,
            Direction::Est => Self::North,
            Direction::South => Self::Est,
            Direction::West => Self::South,
        }
    }

    fn right(self) -> Self {
        match self {
            Direction::North => Direction::Est,
            Direction::Est => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }
}

pub struct Game {
    ants: Vec<Ant>,
    board: Vec<Option<usize>>,
    config: Rc<RefCell<GameConfig>>,
    width: usize,
    height: usize,
}

struct Ant {
    x: usize,
    y: usize,
    direction: Direction,
    id: usize,
    color: Color,
}

impl Game {
    pub fn new(config: Rc<RefCell<GameConfig>>, width: usize, height: usize) -> Self {
        Self {
            ants: vec![],
            board: vec![None; width * height],
            config,
            width,
            height,
        }
    }

    pub fn preview(width: usize, height: usize) -> Self {
        let mut debug_ui = DebugUI::headless();
        let config = GameConfig::new(&mut debug_ui);
        Self {
            ants: vec![],
            board: vec![None; width * height],
            config: Rc::new(RefCell::new(config)),
            width,
            height,
        }
    }

    fn balance_ants(&mut self, canvas: &Canvas) {
        let num_ants = self.config.borrow().num_ants.get();
        match num_ants.cmp(&self.ants.len()) {
            std::cmp::Ordering::Less => self.ants.truncate(num_ants),
            std::cmp::Ordering::Greater => {
                for i in self.ants.len()..num_ants {
                    self.add_ant(i, canvas);
                }
            }
            std::cmp::Ordering::Equal => (),
        }
    }

    fn add_ant(&mut self, id: usize, canvas: &Canvas) {
        let config = self.config.borrow();
        let num_ants = config.num_ants.get();
        let seed = config.seed.get();
        let seed_offset = (seed as f32 * 137.508) % 360.0;
        let hue = if num_ants > 0 {
            (id as f32 * 360.0 / num_ants as f32 + seed_offset) % 360.0
        } else {
            0.0
        };
        let color = hue_to_rgb(
            hue,
            config.ant_color_saturation.get(),
            config.ant_color_brightness.get(),
        );
        let width = canvas.width();
        let screen_height = canvas.screen_height();
        let ant = Ant {
            x: ((width - 1) as f32 * config.start_x_rel.get()) as usize,
            y: ((screen_height - 1) as f32 * config.start_y_rel.get()) as usize,
            direction: Direction::default(),
            id,
            color,
        };
        self.ants.push(ant);
    }
}

impl Simulation for Game {
    fn step(&mut self, canvas: &mut Canvas) {
        self.balance_ants(canvas);
        let config = self.config.borrow();
        // (height, width) — indices are swapped when passing to board/move APIs
        let canvas_size = (self.height, self.width);
        assert!(canvas_size.0 > 0, "Can't draw on a canvas of height 0 !");
        assert!(canvas_size.1 > 0, "Can't draw on a canvas of width 0 !");
        for ant in &mut self.ants {
            let current_cell_state = self.board[ant.x * canvas_size.0 + ant.y];
            let new_cell_color = match current_cell_state {
                None => {
                    ant.direction = ant.direction.right();
                    self.board[ant.x * canvas_size.0 + ant.y] = Some(ant.id);
                    ant.color
                }
                Some(_) => {
                    ant.direction = ant.direction.left();
                    self.board[ant.x * canvas_size.0 + ant.y] = None;
                    let bg = config.common_cell_color.get();
                    Color::Rgb {
                        r: bg.r,
                        g: bg.g,
                        b: bg.b,
                    }
                }
            };
            canvas.fill_rect(ant.x, ant.y, new_cell_color);
            ant.move_forward(canvas_size.1, canvas_size.0);
        }
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.board = vec![None; new_width * new_height];
        for ant in &mut self.ants {
            ant.x = ant.x.min(new_width.saturating_sub(1));
            ant.y = ant.y.min(new_height.saturating_sub(1));
        }
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
        self.board.fill(None);
    }

    fn bg_color(&self) -> Color {
        let c = self.config.borrow();
        let bg = c.common_cell_color.get();
        Color::Rgb {
            r: bg.r,
            g: bg.g,
            b: bg.b,
        }
    }
}

impl Ant {
    fn move_forward(&mut self, board_width: usize, board_height: usize) {
        match self.direction {
            Direction::North => {
                if self.y < board_height - 1 {
                    self.y += 1
                } else {
                    self.y = 0
                }
            }
            Direction::Est => {
                if self.x < board_width - 1 {
                    self.x += 1
                } else {
                    self.x = 0
                }
            }
            Direction::South => {
                if self.y > 0 {
                    self.y -= 1
                } else {
                    self.y = board_height - 1
                }
            }
            Direction::West => {
                if self.x > 0 {
                    self.x -= 1
                } else {
                    self.x = board_width - 1
                }
            }
        }
    }
}

fn hue_to_rgb(hue: f32, saturation: f32, lightness: f32) -> Color {
    let s = saturation;
    let l = lightness;

    let c = (1.0 - (2.0f32 * l - 1.0).abs()) * s;
    let h_prime = hue / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r_temp, g_temp, b_temp) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else if (5.0..=6.0).contains(&h_prime) {
        (c, 0.0, x)
    } else {
        (0.0, 0.0, 0.0)
    };

    let r = ((r_temp + m) * 255.0).round() as u8;
    let g = ((g_temp + m) * 255.0).round() as u8;
    let b = ((b_temp + m) * 255.0).round() as u8;

    Color::Rgb { r, g, b }
}
