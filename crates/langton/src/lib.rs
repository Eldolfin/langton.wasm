use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugUI, Param, ParamParam, StepCounter};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub async fn start_langton_ant() {
    console_error_panic_hook::set_once();
    let mut debug_ui = DebugUI::new("Langton's ant parameters");
    debug_ui.start_section("Canvas");
    let start_x_rel = debug_ui.param(ParamParam {
        name: "start x",
        default_value: 0.80,
        step_size: 0.01,
        needs_restart: true,
        ..Default::default()
    });
    let start_y_rel = debug_ui.param(ParamParam {
        name: "start y",
        default_value: 0.75,
        step_size: 0.01,
        needs_restart: true,
        ..Default::default()
    });
    let alpha_retention_factor = debug_ui.param(ParamParam {
        name: "alpha retention",
        default_value: 251,
        range: 0..=255,
        ..Default::default()
    });

    debug_ui.start_section("Animation Speed");
    let final_steps_per_frame = debug_ui.param(ParamParam {
        name: "final speed",
        default_value: 0.2,
        range: 0.0..=1000.0,
        scale: debug_ui::Scale::Logarithmic,
        ..Default::default()
    });
    let speedup_frames = debug_ui.param(ParamParam {
        name: "speedup frames",
        default_value: 1300,
        range: 0..=1500,
        ..Default::default()
    });

    debug_ui.start_section("Ants");
    let num_ants = debug_ui.param(ParamParam {
        name: "number of ants",
        default_value: 2,
        range: 1..=1000,
        scale: debug_ui::Scale::Logarithmic,
        ..Default::default()
    });
    let ant_color_saturation = debug_ui.param(ParamParam {
        name: "ant color saturation",
        default_value: 0.3,
        range: 0.0..=1.0,
        step_size: 0.01,
        ..Default::default()
    });
    let ant_color_brightness = debug_ui.param(ParamParam {
        name: "ant color brightness",
        default_value: 0.7,
        range: 0.0..=1.0,
        step_size: 0.01,
        ..Default::default()
    });

    debug_ui.start_section("Visual");
    let cell_size = debug_ui.param(ParamParam {
        name: "cell size",
        default_value: 20,
        range: 1..=50,
        ..Default::default()
    });
    let cell_border_size = debug_ui.param(ParamParam {
        name: "cell border size",
        default_value: 1,
        range: 0..=5,
        ..Default::default()
    });
    let white_color_r = debug_ui.param(ParamParam {
        name: "white color red",
        default_value: 30,
        range: 0..=255,
        ..Default::default()
    });
    let white_color_g = debug_ui.param(ParamParam {
        name: "white color green",
        default_value: 30,
        range: 0..=255,
        ..Default::default()
    });
    let white_color_b = debug_ui.param(ParamParam {
        name: "white color blue",
        default_value: 30,
        range: 0..=255,
        ..Default::default()
    });

    debug_ui.start_section("Advanced");
    let speed_ease_in_power = debug_ui.param(ParamParam {
        name: "speed ease-in power",
        default_value: 2.5,
        range: 1.0..=10.0,
        step_size: 0.1,
        ..Default::default()
    });

    debug_ui.link(
        "About this animation",
        "https://codeberg.org/eldolfin/langton.wasm",
    );
    let game_config = GameConfig {
        num_ants,
        final_steps_per_frame,
        speedup_frames,
        start_x_rel,
        start_y_rel,
        alpha_retention_factor,
        ant_color_saturation,
        ant_color_brightness,
        white_color_r,
        white_color_g,
        white_color_b,
        speed_ease_in_power,
    };
    let cell_border_size = Rc::new(RefCell::new(cell_border_size));
    let cell_size = Rc::new(RefCell::new(cell_size));
    let config = Rc::new(RefCell::new(game_config));
    let step_counter = Rc::new(RefCell::new(debug_ui.step_counter()));
    let debug_ui = Rc::new(RefCell::new(debug_ui));
    loop {
        step_counter.borrow_mut().reset();
        let canvas = Canvas::new(cell_border_size.clone(), cell_size.clone());
        let debug_ui_ref = debug_ui.clone();
        let should_restart = Box::new(move || debug_ui_ref.borrow_mut().should_restart());
        Game::new(config.clone())
            .run(canvas, should_restart, step_counter.clone())
            .await;
    }
}

struct GameConfig {
    num_ants: Param<usize>,
    final_steps_per_frame: Param<f64>,
    speedup_frames: Param<usize>,
    start_x_rel: Param<f32>,
    start_y_rel: Param<f32>,
    alpha_retention_factor: Param<u8>,
    ant_color_saturation: Param<f32>,
    ant_color_brightness: Param<f32>,
    white_color_r: Param<u8>,
    white_color_g: Param<u8>,
    white_color_b: Param<u8>,
    speed_ease_in_power: Param<f64>,
}

struct Game {
    ants: Vec<Ant>,
    config: Rc<RefCell<GameConfig>>,
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
    fn new(config: Rc<RefCell<GameConfig>>) -> Self {
        Self {
            ants: vec![],
            config,
        }
    }

    /// An ease-in I felt satisfying enough by trial and error
    fn shit_ease_in(inp: f64, power: f64) -> f64 {
        let out = inp.powf(power);
        (out + 0.005).clamp(0.0, 1.0)
    }

    async fn run(
        mut self,
        canvas: Canvas,
        should_stop: Box<dyn Fn() -> bool>,
        step_counter: Rc<RefCell<StepCounter>>,
    ) {
        let mut prev_canvas_size = (canvas.height(), canvas.width());
        let mut board = vec![None::<usize>; prev_canvas_size.0 * prev_canvas_size.1];
        let mut step_accumulator = 0.0;
        let mut frame_counter = 0;
        let animation = move |canvas: &mut Canvas| {
            self.balance_ants(canvas);
            let mut config = self.config.borrow_mut();
            frame_counter += 1;
            let ratio = (frame_counter as f64 / config.speedup_frames.get() as f64).clamp(0.0, 1.0);
            let ratio = Self::shit_ease_in(ratio, config.speed_ease_in_power.get());
            let step = config.final_steps_per_frame.get() * ratio;
            step_accumulator += step;
            let canvas_size = (canvas.height(), canvas.width());
            if canvas_size != prev_canvas_size {
                prev_canvas_size = canvas_size;
                board = vec![None::<usize>; canvas_size.0 * canvas_size.1];
                for ant in &mut self.ants {
                    ant.x = std::cmp::min(ant.x, canvas_size.1 - 1);
                    ant.y = std::cmp::min(ant.y, canvas_size.0 - 1);
                }
            }
            let mut steps_this_frame: u64 = 0;
            while step_accumulator >= 1.0 {
                step_accumulator -= 1.0;
                steps_this_frame += 1;

                for ant in &mut self.ants {
                    assert!(canvas_size.0 > 0, "Can't draw on a canvas of height 0 !");
                    assert!(canvas_size.1 > 0, "Can't draw on a canvas of width 0 !");
                    let current_cell_state = board[ant.x * canvas_size.0 + ant.y];
                    let new_cell_color;
                    match current_cell_state {
                        None => {
                            // Was white
                            ant.direction = ant.direction.right();
                            board[ant.x * canvas_size.0 + ant.y] = Some(ant.id);
                            new_cell_color = ant.color;
                        }
                        Some(_) => {
                            // Was black/colored by an ant
                            ant.direction = ant.direction.left();
                            board[ant.x * canvas_size.0 + ant.y] = None;
                            new_cell_color = Color::Rgb {
                                r: config.white_color_r.get(),
                                g: config.white_color_g.get(),
                                b: config.white_color_b.get(),
                            };
                        }
                    }
                    canvas.fill_rect(ant.x, ant.y, new_cell_color);
                    ant.move_forward(canvas_size.1, canvas_size.0);
                }
            }

            step_counter.borrow_mut().add_steps(steps_this_frame);

            canvas.fill_canvas(config.alpha_retention_factor.get());

            should_stop()
        };
        let canvas = Rc::new(RefCell::new(canvas));
        Canvas::play_animation(canvas, animation).await;
    }

    fn balance_ants(&mut self, canvas: &mut Canvas) {
        let num_ants = self.config.borrow_mut().num_ants.get();
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

    fn add_ant(&mut self, id: usize, canvas: &mut Canvas) {
        let mut config = self.config.borrow_mut();
        let num_ants = config.num_ants.get();
        let hue = if num_ants > 0 {
            (id as f32 * 360.0) / num_ants as f32
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
    let s = saturation; // Saturation
    let l = lightness; // Lightness

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
