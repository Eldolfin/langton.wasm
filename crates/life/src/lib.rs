use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugColor, DebugUI, Param};
use engine::Simulation;
use engine_macros::SimulationConfig;

#[derive(SimulationConfig)]
pub struct LifeConfig {
    #[param(
        section = "Life",
        name = "density",
        default = "0.30",
        range = "0.0..=1.0",
        step = 0.01,
        needs_restart
    )]
    pub density: Param<f32>,
    #[param(
        section = "Visual",
        name = "cell size",
        default = "8",
        range = "1..=40"
    )]
    pub cell_size: Param<usize>,
    #[param(name = "cell border size", default = "0", range = "0..=5")]
    pub cell_border_size: Param<usize>,
    #[param(
        name = "alive color",
        default = "DebugColor { r: 120, g: 220, b: 160 }",
        color
    )]
    pub alive_color: Param<DebugColor>,
    #[param(
        name = "background color",
        default = "DebugColor { r: 13, g: 17, b: 23 }",
        color
    )]
    pub background_color: Param<DebugColor>,
    #[param(
        section = "Advanced",
        name = "seed",
        default = "1",
        range = "1..=4294967295",
        needs_restart
    )]
    pub seed: Param<u32>,
}

pub const LIFE_PRESETS: &[(&str, &str)] = &[
    (
        "Primordial soup",
        "density=0.30&final_speed=6&alpha_retention=255",
    ),
    (
        "Sparse gliders",
        "density=0.12&final_speed=4&alpha_retention=255",
    ),
    (
        "Dense chaos",
        "density=0.50&final_speed=10&alpha_retention=255",
    ),
    (
        "Ghost trails",
        "density=0.30&final_speed=6&alpha_retention=235",
    ),
];

pub struct LifeSim {
    config: Rc<RefCell<LifeConfig>>,
    width: usize,
    height: usize,
    board: Vec<bool>,
    // Scratch buffer for the next generation; swapped with `board` each step.
    next: Vec<bool>,
    // Snapshot of the board before the last `next_generation`, used to diff for drawing.
    prev: Vec<bool>,
    rng_state: u32,
}

impl LifeSim {
    pub fn new(config: Rc<RefCell<LifeConfig>>, width: usize, height: usize) -> Self {
        let seed = config.borrow().seed.get();
        let mut sim = Self {
            config,
            width,
            height,
            board: Vec::new(),
            next: Vec::new(),
            prev: Vec::new(),
            rng_state: nonzero_seed(seed),
        };
        sim.reseed_soup();
        sim
    }

    pub fn preview(width: usize, height: usize) -> Self {
        let mut debug_ui = DebugUI::headless();
        let config = LifeConfig::new(&mut debug_ui);
        Self::new(Rc::new(RefCell::new(config)), width, height)
    }

    // xorshift32: small, deterministic, no_std-friendly. Seed 0 is invalid for xorshift,
    // hence nonzero_seed() at construction.
    fn next_rand(&mut self) -> u32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        x
    }

    /// (Re)allocate boards at the current size and fill `board` from a random soup:
    /// each cell is alive with probability = density. Resets the RNG from config.seed first.
    fn reseed_soup(&mut self) {
        let (density, seed) = {
            let cfg = self.config.borrow();
            (cfg.density.get().clamp(0.0, 1.0), cfg.seed.get())
        };
        self.rng_state = nonzero_seed(seed);
        let cells = self.width * self.height;
        self.board = vec![false; cells];
        self.next = vec![false; cells];
        self.prev = vec![false; cells];
        // u32::MAX as f32 threshold: cell alive when rand/MAX < density.
        let threshold = density * (u32::MAX as f32);
        for i in 0..cells {
            self.board[i] = (self.next_rand() as f32) < threshold;
        }
    }

    /// Count live neighbours of (x, y) with toroidal (wrapping) edges.
    fn live_neighbours(&self, x: usize, y: usize) -> u8 {
        let w = self.width;
        let h = self.height;
        let mut count: u8 = 0;
        // Wrap offsets via modular arithmetic; +w/+h keeps the subtraction non-negative.
        let xs = [(x + w - 1) % w, x, (x + 1) % w];
        let ys = [(y + h - 1) % h, y, (y + 1) % h];
        for &ny in &ys {
            for &nx in &xs {
                if nx == x && ny == y {
                    continue;
                }
                if self.board[ny * w + nx] {
                    count += 1;
                }
            }
        }
        count
    }

    /// Conway B3/S23 on a toroidal grid. Computes the next generation into `next`,
    /// snapshots the current board into `prev`, then swaps `board`/`next`.
    /// Pure: never touches the Canvas.
    pub fn next_generation(&mut self) {
        let w = self.width;
        let h = self.height;
        if w == 0 || h == 0 {
            return;
        }
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let alive = self.board[idx];
                let n = self.live_neighbours(x, y);
                // Birth on exactly 3; survival on 2 or 3; death otherwise.
                self.next[idx] = matches!((alive, n), (true, 2) | (true, 3) | (false, 3));
            }
        }
        std::mem::swap(&mut self.prev, &mut self.board);
        std::mem::swap(&mut self.board, &mut self.next);
    }
}

impl Simulation for LifeSim {
    fn step(&mut self, canvas: &mut Canvas) {
        self.next_generation();

        let (alive, bg) = {
            let cfg = self.config.borrow();
            (cfg.alive_color.get(), cfg.background_color.get())
        };
        let alive_color = Color::Rgb {
            r: alive.r,
            g: alive.g,
            b: alive.b,
        };
        let bg_color = Color::Rgb {
            r: bg.r,
            g: bg.g,
            b: bg.b,
        };

        // Draw only cells that changed this generation to keep the draw queue small.
        let w = self.width;
        let h = self.height;
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let now = self.board[idx];
                if now == self.prev[idx] {
                    continue;
                }
                if x < w && y < h {
                    canvas.fill_rect(x, y, if now { alive_color } else { bg_color });
                }
            }
        }
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.reseed_soup();
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
        self.reseed_soup();
    }

    fn bg_color(&self) -> Color {
        let bg = self.config.borrow().background_color.get();
        Color::Rgb {
            r: bg.r,
            g: bg.g,
            b: bg.b,
        }
    }
}

fn nonzero_seed(seed: u32) -> u32 {
    if seed == 0 { 1 } else { seed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // Build a config directly from `Param::fixed`, mirroring the `#[param(default=..)]`
    // values. We deliberately avoid `DebugUI::headless()` + `LifeConfig::new` here:
    // those touch `web_sys`/js-sys statics (via `document()`), which panic on the host
    // test target. Direct construction keeps these tests pure and host-runnable.
    fn make_config() -> LifeConfig {
        LifeConfig {
            density: Param::fixed(0.30),
            cell_size: Param::fixed(8),
            cell_border_size: Param::fixed(0),
            alive_color: Param::fixed(DebugColor {
                r: 120,
                g: 220,
                b: 160,
            }),
            background_color: Param::fixed(DebugColor {
                r: 13,
                g: 17,
                b: 23,
            }),
            seed: Param::fixed(1),
        }
    }

    fn make_sim(width: usize, height: usize) -> LifeSim {
        LifeSim::new(Rc::new(RefCell::new(make_config())), width, height)
    }

    /// Overwrite the board from a row-major list of bools (true = alive).
    fn set_board(sim: &mut LifeSim, cells: &[bool]) {
        assert_eq!(cells.len(), sim.width * sim.height);
        sim.board.copy_from_slice(cells);
    }

    fn alive_coords(sim: &LifeSim) -> Vec<(usize, usize)> {
        let w = sim.width;
        (0..sim.board.len())
            .filter(|&i| sim.board[i])
            .map(|i| (i % w, i / w))
            .collect()
    }

    #[test]
    fn test_blinker_oscillates_period_2() {
        // 5x5 toroidal grid, horizontal 1x3 line of live cells at row 2.
        let mut sim = make_sim(5, 5);
        let mut cells = vec![false; 25];
        let w = 5;
        cells[2 * w + 1] = true;
        cells[2 * w + 2] = true;
        cells[2 * w + 3] = true;
        set_board(&mut sim, &cells);

        // After one gen: vertical line (column 2, rows 1..=3).
        sim.next_generation();
        let mut vertical = alive_coords(&sim);
        vertical.sort_unstable();
        assert_eq!(vertical, vec![(2, 1), (2, 2), (2, 3)]);

        // After another gen: back to the original horizontal line (period 2).
        sim.next_generation();
        let mut horizontal = alive_coords(&sim);
        horizontal.sort_unstable();
        assert_eq!(horizontal, vec![(1, 2), (2, 2), (3, 2)]);
    }

    #[test]
    fn test_block_is_stable_still_life() {
        // 4x4 grid, 2x2 block at (1,1)-(2,2): a still life under B3/S23.
        let mut sim = make_sim(4, 4);
        let w = 4;
        let mut cells = vec![false; 16];
        cells[w + 1] = true;
        cells[w + 2] = true;
        cells[2 * w + 1] = true;
        cells[2 * w + 2] = true;
        set_board(&mut sim, &cells);

        let before = sim.board.clone();
        sim.next_generation();
        assert_eq!(sim.board, before, "2x2 block must be stable");
        sim.next_generation();
        assert_eq!(sim.board, before, "2x2 block must remain stable");
    }

    #[test]
    fn test_isolated_cell_dies() {
        // A single live cell with zero neighbours dies (underpopulation).
        let mut sim = make_sim(5, 5);
        let mut cells = vec![false; 25];
        cells[2 * 5 + 2] = true;
        set_board(&mut sim, &cells);

        sim.next_generation();
        assert!(sim.board.iter().all(|&c| !c), "isolated live cell must die");
    }

    #[test]
    fn test_deterministic_seeding() {
        // Same seed (default config) → identical soup boards.
        let a = make_sim(40, 30);
        let b = make_sim(40, 30);
        assert_eq!(a.board, b.board);
    }

    #[rstest]
    #[case(1, 1)]
    #[case(10, 10)]
    #[case(40, 30)]
    #[case(7, 13)]
    fn test_next_generation_preserves_length(#[case] w: usize, #[case] h: usize) {
        let mut sim = make_sim(w, h);
        assert_eq!(sim.board.len(), w * h);
        sim.next_generation();
        assert_eq!(
            sim.board.len(),
            w * h,
            "board length must equal width*height"
        );
    }

    #[test]
    fn test_toroidal_wrap_neighbours() {
        // Corner cell (0,0) on a 4x4 grid: with only (0,0) alive, its own neighbour
        // count is 0 (wrapping doesn't make a cell its own neighbour).
        let mut sim = make_sim(4, 4);
        let mut cells = vec![false; 16];
        let w = 4;
        cells[0] = true;
        set_board(&mut sim, &cells);
        assert_eq!(sim.live_neighbours(0, 0), 0);

        // Left-edge cell (0,0) must see live cells across the wrap seam: the opposite
        // column (w-1) and the opposite row (bottom) are diagonal/orthogonal neighbours.
        let mut wrap = make_sim(4, 4);
        let mut cells = vec![false; 16];
        cells[w - 1] = true; // (3,0): wraps to be the left neighbour of (0,0)
        cells[(4 - 1) * w] = true; // (0,3): wraps to be the top neighbour of (0,0)
        set_board(&mut wrap, &cells);
        assert_eq!(wrap.live_neighbours(0, 0), 2);
    }

    #[test]
    fn test_toroidal_blinker_wraps_across_seam() {
        // A vertical 3-line straddling the top/bottom seam on a 5x5 torus is a blinker:
        // column 0, rows {4, 0, 1} → after one gen it becomes the horizontal triple
        // centred on (0,0): cells (4,0), (0,0), (1,0).
        let mut sim = make_sim(5, 5);
        let w = 5;
        let mut cells = vec![false; 25];
        cells[4 * w] = true; // (0,4)
        cells[0] = true; // (0,0)
        cells[w] = true; // (0,1)
        set_board(&mut sim, &cells);

        sim.next_generation();
        let mut got = alive_coords(&sim);
        got.sort_unstable();
        assert_eq!(got, vec![(0, 0), (1, 0), (4, 0)]);
    }

    #[test]
    fn test_new_allocates_full_soup() {
        // `LifeSim::preview` is exercised at runtime; it routes through DebugUI/js-sys
        // and cannot run on the host target, so we test the equivalent `new` path here.
        let sim = make_sim(50, 40);
        assert_eq!(sim.width, 50);
        assert_eq!(sim.height, 40);
        assert_eq!(sim.board.len(), 50 * 40);
        assert_eq!(sim.next.len(), 50 * 40);
        assert_eq!(sim.prev.len(), 50 * 40);
        // With density 0.30 over 2000 cells, some cells live and some are dead.
        let live = sim.board.iter().filter(|&&c| c).count();
        assert!(live > 0 && live < sim.board.len(), "soup should be mixed");
    }

    #[test]
    fn test_on_canvas_resize_rebuilds_boards() {
        let mut sim = make_sim(10, 10);
        sim.on_canvas_resize(20, 8);
        assert_eq!(sim.width, 20);
        assert_eq!(sim.height, 8);
        assert_eq!(sim.board.len(), 20 * 8);
        assert_eq!(sim.next.len(), 20 * 8);
        assert_eq!(sim.prev.len(), 20 * 8);
    }

    #[test]
    fn test_bg_color_uses_config() {
        let sim = make_sim(20, 20);
        match sim.bg_color() {
            Color::Rgb { r, g, b } => assert_eq!((r, g, b), (13, 17, 23)),
            other => panic!("expected Rgb, got {other:?}"),
        }
    }

    #[test]
    fn test_nonzero_seed_guard() {
        assert_eq!(nonzero_seed(0), 1);
        assert_eq!(nonzero_seed(1), 1);
        assert_eq!(nonzero_seed(42), 42);
        assert_eq!(nonzero_seed(u32::MAX), u32::MAX);
    }

    #[test]
    fn test_rng_is_deterministic() {
        let mut a = make_sim(10, 10);
        let mut b = make_sim(10, 10);
        for _ in 0..10 {
            assert_eq!(a.next_rand(), b.next_rand());
        }
    }
}
