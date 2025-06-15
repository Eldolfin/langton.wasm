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
    let alpha_retention_factor = debug_ui.param(ParamParam {
        name: "alpha retention",
        default_value: 250,
        range: 0..255,
        ..Default::default()
    });

    let num_ants = debug_ui.param(ParamParam {
        name: "number of ants",
        default_value: 2,
        range: 1..1000,
        scale: debug_ui::Scale::Logarithmic,
        ..Default::default()
    });

    Game::new(GameConfig {
        final_steps_per_frame,
        speedup_frames,
        start_x_rel,
        start_y_rel,
        alpha_retention_factor,
        num_ants,
    })
    .run()
    .await;
}

struct GameConfig {
    num_ants: Param<usize>,
    final_steps_per_frame: Param<f64>,
    speedup_frames: Param<usize>,
    start_x_rel: Param<f32>,
    start_y_rel: Param<f32>,
    alpha_retention_factor: Param<u8>,
}

struct Game {
    canvas: Canvas,
    /// indexed by x, y
    board: Vec<Vec<Option<usize>>>,
    ants: Vec<Ant>,
    config: GameConfig,
}

struct Ant {
    x: usize,
    y: usize,
    direction: Direction,
    id: usize,
    color: Color,
}

#[derive(Debug, Clone, Copy, Default)]
enum Direction {
    #[default]
    North,
    Est,
    South,
    West,
}

impl Game {
    fn new(mut config: GameConfig) -> Self {
        let canvas = Canvas::create_bg().unwrap().with_cell_size(10.);
        let mut ants = Vec::new();
        let num_ants_val = config.num_ants.get();
        for i in 0..num_ants_val {
            let id = i;
            let hue = if num_ants_val > 0 {
                (id as f32 * 360.0) / num_ants_val as f32
            } else {
                0.0
            };
            let color = hue_to_rgb(hue);

            let ant = Ant {
                x: ((canvas.width() - 1) as f32 * config.start_x_rel.get()) as usize,
                y: ((canvas.screen_height() - 1) as f32 * config.start_y_rel.get()) as usize,
                direction: Direction::default(),
                id,
                color,
            };
            ants.push(ant);
        }
        let board = vec![vec![None; canvas.height()]; canvas.width()];

        Self {
            board,
            canvas,
            ants,
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

                for ant in &mut self.ants {
                    let current_cell_state = self.board[ant.x][ant.y];
                    let new_cell_color;
                    match current_cell_state {
                        None => {
                            // Was white
                            ant.direction = ant.direction.right();
                            self.board[ant.x][ant.y] = Some(ant.id);
                            new_cell_color = ant.color;
                        }
                        Some(_) => {
                            // Was black/colored by an ant
                            ant.direction = ant.direction.left();
                            self.board[ant.x][ant.y] = None;
                            new_cell_color = Color::Named(NamedColor::White);
                        }
                    }
                    canvas.fill_rect(ant.x, ant.y, new_cell_color);
                    ant.move_forward(canvas.width(), canvas.height());
                }
            }

            canvas.fill_canvas(self.config.alpha_retention_factor.get());

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

fn hue_to_rgb(hue: f32) -> Color {
    let s = 1.0; // Saturation
    let l = 0.5; // Lightness

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
        (0.0, 0.0, 0.0) // Should not happen with hue in 0-360
    };

    let r = ((r_temp + m) * 255.0).round() as u8;
    let g = ((g_temp + m) * 255.0).round() as u8;
    let b = ((b_temp + m) * 255.0).round() as u8;

    Color::Rgb { r, g, b }
}

// Removed BoardState enum and its impl blocks

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
