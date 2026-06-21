use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugColor, DebugUI, Param};
use engine::Simulation;
use engine_macros::SimulationConfig;

#[derive(SimulationConfig)]
pub struct CcaConfig {
    #[param(
        section = "Cyclic CA",
        name = "states",
        default = "14",
        range = "2..=24",
        needs_restart
    )]
    pub num_states: Param<usize>,
    #[param(name = "threshold", default = "1", range = "1..=8")]
    pub threshold: Param<usize>,
    #[param(
        section = "Visual",
        name = "cell size",
        default = "6",
        range = "1..=30"
    )]
    pub cell_size: Param<usize>,
    #[param(name = "cell border size", default = "0", range = "0..=5")]
    pub cell_border_size: Param<usize>,
    #[param(
        name = "color saturation",
        default = "0.7",
        range = "0.0..=1.0",
        step = 0.01
    )]
    pub color_saturation: Param<f32>,
    #[param(
        name = "color brightness",
        default = "0.6",
        range = "0.0..=1.0",
        step = 0.01
    )]
    pub color_brightness: Param<f32>,
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

pub const CCA_PRESETS: &[(&str, &str)] = &[
    (
        "Spirals",
        "states=14&threshold=1&final_speed=1.5&speedup_frames=0&alpha_retention=255",
    ),
    (
        "Demon spirals",
        "states=8&threshold=3&final_speed=1&speedup_frames=0&alpha_retention=255",
    ),
    (
        "Lava lamp",
        "states=4&threshold=1&final_speed=1&speedup_frames=0&alpha_retention=255",
    ),
    (
        "Rainbow waves",
        "states=20&threshold=2&final_speed=2&speedup_frames=0&alpha_retention=255",
    ),
];

pub struct CcaSim {
    config: Rc<RefCell<CcaConfig>>,
    width: usize,
    height: usize,
    board: Vec<u8>,
    next: Vec<u8>,
    rng_state: u32,
}

impl CcaSim {
    pub fn new(config: Rc<RefCell<CcaConfig>>, width: usize, height: usize) -> Self {
        let seed = config.borrow().seed.get();
        let mut sim = Self {
            config,
            width,
            height,
            board: Vec::new(),
            next: Vec::new(),
            rng_state: nonzero_seed(seed),
        };
        sim.reseed_board();
        sim
    }

    pub fn preview(width: usize, height: usize) -> Self {
        let mut debug_ui = DebugUI::headless();
        let config = CcaConfig::new(&mut debug_ui);
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

    fn num_states(&self) -> usize {
        self.config.borrow().num_states.get().max(2)
    }

    /// Reset RNG from the configured seed and refill the board with random states.
    fn reseed_board(&mut self) {
        let seed = self.config.borrow().seed.get();
        self.rng_state = nonzero_seed(seed);
        let n = self.num_states() as u32;
        let len = self.width * self.height;
        self.board = vec![0u8; len];
        self.next = vec![0u8; len];
        for i in 0..len {
            self.board[i] = (self.next_rand() % n) as u8;
        }
    }

    /// Pure cyclic-CA transition over the toroidal grid. No Canvas access.
    /// A cell in state `s` advances to `(s + 1) % n` when at least `threshold`
    /// of its 8 Moore neighbours are already in that successor state.
    fn step_cca(&mut self) {
        let (n, threshold) = {
            let cfg = self.config.borrow();
            (cfg.num_states.get().max(2) as u8, cfg.threshold.get())
        };
        let w = self.width;
        let h = self.height;
        if w == 0 || h == 0 {
            return;
        }

        for y in 0..h {
            for x in 0..w {
                let s = self.board[y * w + x];
                let target = (s + 1) % n;
                let mut count = 0usize;
                for dy in [h - 1, 0, 1] {
                    let ny = (y + dy) % h;
                    for dx in [w - 1, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = (x + dx) % w;
                        if self.board[ny * w + nx] == target {
                            count += 1;
                        }
                    }
                }
                self.next[y * w + x] = if count >= threshold { target } else { s };
            }
        }

        std::mem::swap(&mut self.board, &mut self.next);
    }

    fn cell_color(&self, state: u8, n: usize, saturation: f32, brightness: f32) -> Color {
        let hue = (state as f32 / n as f32) * 360.0;
        hue_to_rgb(hue, saturation, brightness)
    }
}

impl Simulation for CcaSim {
    fn step(&mut self, canvas: &mut Canvas) {
        let w = self.width;
        let h = self.height;
        if w == 0 || h == 0 {
            return;
        }
        let (n, saturation, brightness) = {
            let cfg = self.config.borrow();
            (
                cfg.num_states.get().max(2),
                cfg.color_saturation.get(),
                cfg.color_brightness.get(),
            )
        };

        // Compute the next generation into `self.next`, then draw only the cells
        // whose state changed. `step_cca` swaps board/next, so after it runs the
        // new generation lives in `board` and the previous one in `next`.
        self.step_cca();

        let cw = canvas.width();
        let ch = canvas.height();
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let new_state = self.board[idx];
                if new_state != self.next[idx] && x < cw && y < ch {
                    canvas.fill_rect(x, y, self.cell_color(new_state, n, saturation, brightness));
                }
            }
        }
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.reseed_board();
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
        self.reseed_board();
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // Build a config directly from fixed params. The derived `CcaConfig::new`
    // reads the page URL via common::url(), which calls into js-sys and panics
    // on the host test target; constructing the struct by hand keeps these
    // tests pure logic (no js-sys / web-sys / Canvas access).
    fn make_config() -> CcaConfig {
        CcaConfig {
            num_states: Param::fixed(14),
            threshold: Param::fixed(1),
            cell_size: Param::fixed(6),
            cell_border_size: Param::fixed(0),
            color_saturation: Param::fixed(0.7),
            color_brightness: Param::fixed(0.6),
            background_color: Param::fixed(DebugColor {
                r: 13,
                g: 17,
                b: 23,
            }),
            seed: Param::fixed(1),
        }
    }

    fn make_sim(width: usize, height: usize) -> CcaSim {
        CcaSim::new(Rc::new(RefCell::new(make_config())), width, height)
    }

    /// Override num_states / threshold on an existing sim so tests can build
    /// small deterministic boards without depending on the URL-derived defaults.
    /// Reseeds the board afterwards so its contents respect the new state count
    /// (the original fill used the default num_states).
    fn set_rules(sim: &mut CcaSim, num_states: usize, threshold: usize) {
        {
            let mut cfg = sim.config.borrow_mut();
            cfg.num_states = Param::fixed(num_states);
            cfg.threshold = Param::fixed(threshold);
        }
        sim.reseed_board();
    }

    #[test]
    fn test_board_length_matches_dimensions() {
        let sim = make_sim(10, 7);
        assert_eq!(sim.board.len(), 70);
        assert_eq!(sim.next.len(), 70);
    }

    #[test]
    fn test_board_length_preserved_after_step() {
        let mut sim = make_sim(8, 9);
        set_rules(&mut sim, 5, 1);
        for _ in 0..20 {
            sim.step_cca();
            assert_eq!(sim.board.len(), 72);
            assert_eq!(sim.next.len(), 72);
        }
    }

    #[test]
    fn test_cell_advances_when_threshold_met() {
        // 3x3 board, n=4, threshold=3. Center is state 0; surround it with
        // enough cells in state 1 (its successor) to cross the threshold.
        let mut sim = make_sim(3, 3);
        set_rules(&mut sim, 4, 3);
        // All zeros, then place three 1s among the center's neighbours.
        sim.board = vec![0; 9];
        sim.board[0] = 1; // top-left
        sim.board[1] = 1; // top-mid
        sim.board[2] = 1; // top-right
        sim.step_cca();
        // Center (index 4) had 3 neighbours in state 1 -> advances to 1.
        assert_eq!(sim.board[4], 1);
    }

    #[test]
    fn test_cell_stays_when_threshold_not_met() {
        let mut sim = make_sim(3, 3);
        set_rules(&mut sim, 4, 3);
        sim.board = vec![0; 9];
        // Only two neighbours in the successor state (1) -> below threshold 3.
        sim.board[0] = 1;
        sim.board[1] = 1;
        sim.step_cca();
        assert_eq!(sim.board[4], 0, "center must stay in state 0");
    }

    #[test]
    fn test_advance_is_exactly_one_mod_n() {
        // n=3, threshold=1. A center in state 2 with a neighbour in state 0
        // (its successor (2+1)%3) must wrap to 0, never skip.
        let mut sim = make_sim(3, 3);
        set_rules(&mut sim, 3, 1);
        sim.board = vec![2; 9];
        sim.board[0] = 0; // one neighbour of center already at successor state 0
        sim.step_cca();
        assert_eq!(sim.board[4], 0, "state 2 must advance to (2+1)%3 = 0");
    }

    #[test]
    fn test_toroidal_wrap_counts_edge_neighbours() {
        // n=2, threshold=1. Cell (0,0)=0, successor=1. Its left/up neighbours
        // wrap to the far edges; put a single 1 at the bottom-right corner,
        // which is the diagonal wrap-around neighbour of (0,0).
        let mut sim = make_sim(3, 3);
        set_rules(&mut sim, 2, 1);
        sim.board = vec![0; 9];
        sim.board[8] = 1; // bottom-right, wraps to be a neighbour of top-left
        sim.step_cca();
        assert_eq!(
            sim.board[0], 1,
            "top-left must see bottom-right via toroidal wrap and advance"
        );
    }

    #[rstest]
    #[case(2, 1)]
    #[case(3, 2)]
    #[case(5, 1)]
    #[case(14, 1)]
    #[case(24, 8)]
    fn test_states_stay_in_range_after_step(#[case] n: usize, #[case] threshold: usize) {
        let mut sim = make_sim(16, 16);
        set_rules(&mut sim, n, threshold);
        for _ in 0..50 {
            sim.step_cca();
            for &s in &sim.board {
                assert!((s as usize) < n, "state {s} out of range 0..{n}");
            }
        }
    }

    #[test]
    fn test_deterministic_seeding_same_seed_same_board() {
        let a = make_sim(20, 20);
        let b = make_sim(20, 20);
        assert_eq!(a.board, b.board, "same seed must yield identical boards");
    }

    #[test]
    fn test_initial_states_in_range() {
        let sim = make_sim(32, 32);
        let n = sim.num_states();
        for &s in &sim.board {
            assert!((s as usize) < n, "initial state {s} out of range 0..{n}");
        }
    }

    #[test]
    fn test_resize_rebuilds_board() {
        let mut sim = make_sim(10, 10);
        sim.on_canvas_resize(5, 8);
        assert_eq!(sim.width, 5);
        assert_eq!(sim.height, 8);
        assert_eq!(sim.board.len(), 40);
        assert_eq!(sim.next.len(), 40);
    }

    #[test]
    fn test_rng_is_deterministic() {
        let mut a = make_sim(4, 4);
        let mut b = make_sim(4, 4);
        for _ in 0..10 {
            assert_eq!(a.next_rand(), b.next_rand());
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
    fn test_num_states_clamped_to_two() {
        let mut sim = make_sim(4, 4);
        set_rules(&mut sim, 1, 1);
        assert_eq!(sim.num_states(), 2, "num_states must clamp up to 2");
    }

    #[test]
    fn test_new_sets_dimensions_and_fills_board() {
        // Mirrors what preview()/new() guarantee, but without the headless
        // DebugUI path that touches js-sys on the host test target.
        let sim = make_sim(12, 9);
        assert_eq!(sim.width, 12);
        assert_eq!(sim.height, 9);
        assert_eq!(sim.board.len(), 108);
        assert_eq!(sim.next.len(), 108);
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
    fn test_uniform_board_is_stable_below_threshold() {
        // A board entirely in one state with threshold > 0 and n>1 cannot have
        // any neighbours in the successor state, so nothing changes.
        let mut sim = make_sim(6, 6);
        set_rules(&mut sim, 4, 1);
        sim.board = vec![2; 36];
        let before = sim.board.clone();
        sim.step_cca();
        assert_eq!(sim.board, before, "uniform board must be a fixed point");
    }
}
