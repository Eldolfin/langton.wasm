use canvas::{Canvas, Color, NamedColor};
use debug_ui::{DebugUI, Param, ParamParam};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
async fn start() {
    console_error_panic_hook::set_once();
    let mut debug_ui = DebugUI::new("Langton's ant parameters");
    let final_steps_per_frame = debug_ui.param(ParamParam {
        name: "final speed",
        default_value: 12.0,
        range: 0.0..1000.0,
        scale: debug_ui::Scale::Logarithmic,
        ..Default::default()
    });
    let speedup_frames = debug_ui.param(ParamParam {
        name: "speedup frames",
        default_value: 1300,
        range: 0..1500,
        ..Default::default()
    });
    let start_x_rel = debug_ui.param(ParamParam {
        name: "start x",
        default_value: 0.80,
        step_size: 0.01,
        ..Default::default()
    });
    let start_y_rel = debug_ui.param(ParamParam {
        name: "start y",
        default_value: 0.75,
        step_size: 0.01,
        ..Default::default()
    });
    // Updated parameter
    let alpha_retention_factor = debug_ui.param(ParamParam {
        name: "alpha retention",
        default_value: 250,     // u8 value, type u8 inferred
        range: 0u8..=255u8,       // u8 inclusive range for full 0-255 coverage
        ..Default::default()      // Other fields like scale, step_size use default
    });

    Game::new(GameConfig {
        final_steps_per_frame,
        speedup_frames,
        start_x_rel,
        start_y_rel,
        alpha_retention_factor, // Use renamed field
    })
    .run()
    .await;
}

struct GameConfig {
    final_steps_per_frame: Param<f64>,
    speedup_frames: Param<usize>,
    start_x_rel: Param<f32>,
    start_y_rel: Param<f32>,
    alpha_retention_factor: Param<u8>, // Changed to Param<u8>
}

struct Game {
    canvas: Canvas,
    /// indexed by x, y
    board: Vec<Vec<BoardState>>,
    ant: Ant,
    config: GameConfig,
}

struct Ant {
    x: usize,
    y: usize,
    direction: Direction,
}

#[derive(Debug, Clone, Copy, Default)]
enum Direction {
    #[default]
    North,
    Est,
    South,
    West,
}

#[derive(Debug, Clone, Copy, Default)]
enum BoardState {
    #[default]
    White,
    Black,
}

impl Game {
    fn new(mut config: GameConfig) -> Self {
        let canvas = Canvas::get_element_by_id("canvas")
            .unwrap()
            .with_cell_size(10.);
        let ant = Ant {
            x: (canvas.width() as f32 * config.start_x_rel.get()) as usize,
            y: (canvas.height() as f32 * config.start_y_rel.get()) as usize,
            direction: Direction::default(),
        };
        let board = vec![vec![BoardState::default(); canvas.height()]; canvas.width()];

        Self {
            board,
            canvas,
            ant,
            config,
        }
    }

    /// An ease-in I felt satisfying enough by trial and error
    fn shit_ease_in(inp: f64) -> f64 {
        let out = inp * inp * inp * inp;
        (out + 0.005).clamp(0.0, 1.0)
    }

    async fn run(mut self) {
        let mut step_accumulator = 0.0;
        let mut frame_counter = 0;
        let animation = move |canvas: &mut Canvas| {
            frame_counter += 1;
            let ratio =
                (frame_counter as f64 / self.config.speedup_frames.get() as f64).clamp(0.0, 1.0);
            let ratio = Self::shit_ease_in(ratio);
            let step = self.config.final_steps_per_frame.get() * ratio;
            step_accumulator += step;
            while step_accumulator >= 1.0 {
                step_accumulator -= 1.0;
                let at_ant = self.board[self.ant.x][self.ant.y];
                match at_ant {
                    BoardState::White => {
                        self.ant.direction = self.ant.direction.right();
                    }
                    BoardState::Black => {
                        self.ant.direction = self.ant.direction.left();
                    }
                }
                self.board[self.ant.x][self.ant.y] = !at_ant;
                canvas.fill_rect(self.ant.x, self.ant.y, (!at_ant).to_canvas_color());
                self.ant.move_forward(canvas.width(), canvas.height());
            }

            // Updated call to fill_canvas
            // canvas.fill_canvas(self.config.alpha_retention_factor.get() as u8); // Old line with usize
            canvas.fill_canvas(self.config.alpha_retention_factor.get()); // New line, .get() returns u8

            false
        };
        self.canvas.play_animation(animation).await;
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

impl BoardState {
    fn to_canvas_color(self) -> Color {
        match self {
            BoardState::White => Color::Named(NamedColor::White),
            BoardState::Black => Color::Named(NamedColor::Black),
        }
    }
}

impl std::ops::Not for BoardState {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            BoardState::White => Self::Black,
            BoardState::Black => Self::White,
        }
    }
}
