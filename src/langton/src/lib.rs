use canvas::{Canvas, Color, NamedColor};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
async fn start() {
    console_error_panic_hook::set_once();
    let steps_per_frame = 2;
    Game::new(steps_per_frame, 0.80, 0.75).run().await;
}

struct Game {
    canvas: Canvas,
    /// indexed by x, y
    board: Vec<Vec<BoardState>>,
    ant: Ant,
    steps_per_frame: usize,
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
    fn new(steps_per_frame: usize, start_x_rel: f32, start_y_rel: f32) -> Self {
        let canvas = Canvas::get_element_by_id("canvas")
            .unwrap()
            .with_cell_size(10.);
        let ant = Ant {
            x: (canvas.width() as f32 * start_x_rel) as usize,
            y: (canvas.height() as f32 * start_y_rel) as usize,
            direction: Direction::default(),
        };
        let board =
            vec![vec![BoardState::default(); canvas.height() as usize]; canvas.width() as usize];

        Self {
            board,
            canvas,
            ant,
            steps_per_frame,
        }
    }

    async fn run(mut self) {
        let animation = move |canvas: &Canvas| {
            for _ in 0..self.steps_per_frame {
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
                canvas.fill_rect(
                    self.ant.x as u32,
                    self.ant.y as u32,
                    (!at_ant).to_canvas_color(),
                );
                self.ant
                    .move_forward(canvas.width() as usize, canvas.height() as usize);
            }
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
