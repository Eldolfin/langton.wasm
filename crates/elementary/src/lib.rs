use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugColor, DebugUI, Param};
use engine::Simulation;
use engine_macros::SimulationConfig;

#[derive(SimulationConfig)]
pub struct ElementaryConfig {
    #[param(
        section = "Rule",
        name = "rule",
        default = "90",
        range = "0..=255",
        needs_restart
    )]
    pub rule: Param<usize>,
    #[param(name = "random start", default = "0", range = "0..=1", needs_restart)]
    pub random_start: Param<usize>,
    #[param(
        section = "Visual",
        name = "cell size",
        default = "4",
        range = "1..=20"
    )]
    pub cell_size: Param<usize>,
    #[param(name = "cell border size", default = "0", range = "0..=5")]
    pub cell_border_size: Param<usize>,
    #[param(
        name = "on color",
        default = "DebugColor { r: 240, g: 240, b: 240 }",
        color
    )]
    pub on_color: Param<DebugColor>,
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

pub const ELEMENTARY_PRESETS: &[(&str, &str)] = &[
    (
        "Rule 90 (Sierpinski)",
        "rule=90&random_start=0&on_color=%23F0F0F0&final_speed=4&alpha_retention=255",
    ),
    (
        "Rule 30 (chaos)",
        "rule=30&random_start=0&on_color=%23FFC8A0&final_speed=4&alpha_retention=255",
    ),
    (
        "Rule 110",
        "rule=110&random_start=1&on_color=%23A0E0FF&final_speed=4&alpha_retention=255",
    ),
    (
        "Rule 73",
        "rule=73&random_start=1&on_color=%23FFFFFF&final_speed=6&alpha_retention=255",
    ),
];

pub struct ElementarySim {
    config: Rc<RefCell<ElementaryConfig>>,
    width: usize,
    height: usize,
    cur_row: Vec<bool>,
    y: usize,
    rng_state: u32,
}

impl ElementarySim {
    pub fn new(config: Rc<RefCell<ElementaryConfig>>, width: usize, height: usize) -> Self {
        let seed = config.borrow().seed.get();
        let mut sim = Self {
            config,
            width,
            height,
            cur_row: Vec::new(),
            y: 0,
            rng_state: nonzero_seed(seed),
        };
        sim.cur_row = sim.init_row();
        sim
    }

    pub fn preview(width: usize, height: usize) -> Self {
        let mut debug_ui = DebugUI::headless();
        let config = ElementaryConfig::new(&mut debug_ui);
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

    /// Build the first row: either a single live cell centered, or a random
    /// row (random when `random_start` != 0). Random uses the seeded RNG so the
    /// same seed yields an identical row. Pure logic, no Canvas access.
    fn init_row(&mut self) -> Vec<bool> {
        let (random, width) = (self.config.borrow().random_start.get() != 0, self.width);
        let mut row = vec![false; width];
        if width == 0 {
            return row;
        }
        if random {
            for cell in row.iter_mut() {
                *cell = self.next_rand() & 1 == 1;
            }
        } else {
            row[width / 2] = true;
        }
        row
    }

    /// Apply the current `rule` to `row` with toroidal wrap at the ends,
    /// producing the next generation. Pure logic, no Canvas access.
    fn next_row(&self, row: &[bool]) -> Vec<bool> {
        let n = row.len();
        if n == 0 {
            return Vec::new();
        }
        let rule = self.config.borrow().rule.get() as u8;
        let mut next = vec![false; n];
        for i in 0..n {
            let left = row[(i + n - 1) % n];
            let center = row[i];
            let right = row[(i + 1) % n];
            let pattern = ((left as u8) << 2) | ((center as u8) << 1) | (right as u8);
            next[i] = (rule >> pattern) & 1 == 1;
        }
        next
    }
}

impl Simulation for ElementarySim {
    fn step(&mut self, canvas: &mut Canvas) {
        let (on, bg) = {
            let cfg = self.config.borrow();
            (cfg.on_color.get(), cfg.background_color.get())
        };
        let on_color = Color::Rgb {
            r: on.r,
            g: on.g,
            b: on.b,
        };
        let bg_color = Color::Rgb {
            r: bg.r,
            g: bg.g,
            b: bg.b,
        };

        let y = self.y;
        if y < self.height {
            for (x, &alive) in self.cur_row.iter().enumerate() {
                if x < self.width {
                    canvas.fill_rect(x, y, if alive { on_color } else { bg_color });
                }
            }
        }

        let next = self.next_row(&self.cur_row);
        self.y += 1;
        if self.y >= self.height {
            self.y = 0;
            self.cur_row = self.init_row();
        } else {
            self.cur_row = next;
        }
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.y = 0;
        self.cur_row = self.init_row();
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
        let seed = self.config.borrow().seed.get();
        self.rng_state = nonzero_seed(seed);
        self.y = 0;
        self.cur_row = self.init_row();
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

    /// Build a config directly from `Param::fixed` values, matching the declared
    /// defaults (rule 90, single-cell start, seed 1). This deliberately avoids
    /// `ElementaryConfig::new`/`DebugUI`, which read `common::url()` and touch
    /// js-sys statics that panic on the host (non-wasm) test target.
    fn make_config() -> ElementaryConfig {
        ElementaryConfig {
            rule: Param::fixed(90),
            random_start: Param::fixed(0),
            cell_size: Param::fixed(4),
            cell_border_size: Param::fixed(0),
            on_color: Param::fixed(DebugColor {
                r: 240,
                g: 240,
                b: 240,
            }),
            background_color: Param::fixed(DebugColor {
                r: 13,
                g: 17,
                b: 23,
            }),
            seed: Param::fixed(1),
        }
    }

    /// Headless sim built from `make_config`. Pure-logic helper: never calls
    /// `step`/Canvas/js-sys.
    fn make_sim(width: usize, height: usize) -> ElementarySim {
        ElementarySim::new(Rc::new(RefCell::new(make_config())), width, height)
    }

    fn set_rule(sim: &ElementarySim, rule: usize) {
        sim.config.borrow_mut().rule = Param::fixed(rule);
    }

    fn set_random_start(sim: &ElementarySim, v: usize) {
        sim.config.borrow_mut().random_start = Param::fixed(v);
    }

    #[test]
    fn test_next_row_length_matches_input() {
        let sim = make_sim(11, 11);
        for len in [1usize, 2, 5, 11, 33] {
            let row = vec![false; len];
            assert_eq!(sim.next_row(&row).len(), len);
        }
    }

    #[test]
    fn test_next_row_empty_input() {
        let sim = make_sim(0, 0);
        assert!(sim.next_row(&[]).is_empty());
    }

    #[test]
    fn test_rule_90_single_cell_lights_neighbors_clears_center() {
        // Rule 90 = XOR of left and right neighbors. A single live cell at the
        // center must produce two live cells at the immediate neighbors and a
        // dead center on the next row. Width 7, center at index 3.
        let sim = make_sim(7, 7);
        set_rule(&sim, 90);
        let mut row = vec![false; 7];
        row[3] = true;
        let next = sim.next_row(&row);
        let expected = [false, false, true, false, true, false, false];
        assert_eq!(next, expected, "rule 90 single-cell expansion");
    }

    #[test]
    fn test_rule_90_xor_pattern_full_row() {
        // Rule 90: next[i] = left XOR right. Verify against a hand-computed case.
        // Width 5, toroidal. row = [T, F, T, F, F].
        // i0: left=row[4]=F, right=row[1]=F -> F
        // i1: left=row[0]=T, right=row[2]=T -> F
        // i2: left=row[1]=F, right=row[3]=F -> F
        // i3: left=row[2]=T, right=row[4]=F -> T
        // i4: left=row[3]=F, right=row[0]=T -> T
        let sim = make_sim(5, 5);
        set_rule(&sim, 90);
        let row = [true, false, true, false, false];
        let next = sim.next_row(&row);
        assert_eq!(next, [false, false, false, true, true]);
    }

    #[test]
    fn test_rule_0_yields_all_dead() {
        let sim = make_sim(9, 9);
        set_rule(&sim, 0);
        // Any input -> all dead (rule 0 has no set bits).
        let inputs = [vec![false; 9], vec![true; 9], {
            let mut r = vec![false; 9];
            r[4] = true;
            r
        }];
        for input in inputs {
            assert!(sim.next_row(&input).iter().all(|&c| !c));
        }
    }

    #[test]
    fn test_rule_255_yields_all_live() {
        let sim = make_sim(9, 9);
        set_rule(&sim, 255);
        // Any input -> all live (rule 255 has every bit set).
        let inputs = [vec![false; 9], vec![true; 9], {
            let mut r = vec![false; 9];
            r[4] = true;
            r
        }];
        for input in inputs {
            assert!(sim.next_row(&input).iter().all(|&c| c));
        }
    }

    #[test]
    fn test_toroidal_wrap_at_ends() {
        // Rule 90 (XOR neighbors) on a width-3 row with only the center live:
        // [F, T, F].
        // i0: left=row[2]=F, right=row[1]=T -> T
        // i1: left=row[0]=F, right=row[2]=F -> F
        // i2: left=row[1]=T, right=row[0]=F -> T
        let sim = make_sim(3, 3);
        set_rule(&sim, 90);
        let row = [false, true, false];
        assert_eq!(sim.next_row(&row), [true, false, true]);

        // Single live cell at index 0 exercises the left wrap (left=row[n-1]).
        // [T, F, F]:
        // i0: left=row[2]=F, right=row[1]=F -> F
        // i1: left=row[0]=T, right=row[2]=F -> T
        // i2: left=row[1]=F, right=row[0]=T -> T
        let row = [true, false, false];
        assert_eq!(sim.next_row(&row), [false, true, true]);
    }

    #[rstest]
    #[case(7, 3)]
    #[case(8, 4)]
    #[case(20, 10)]
    #[case(21, 10)]
    fn test_single_cell_init_row_has_one_live_centered(#[case] width: usize, #[case] mid: usize) {
        let mut sim = make_sim(width, width);
        set_random_start(&sim, 0);
        let row = sim.init_row();
        assert_eq!(row.len(), width);
        assert_eq!(
            row.iter().filter(|&&c| c).count(),
            1,
            "exactly one live cell"
        );
        assert!(row[mid], "live cell at center index {mid}");
    }

    #[test]
    fn test_random_init_row_is_deterministic_for_same_seed() {
        // Two independently-constructed sims with identical config (seed 1) must
        // produce byte-identical random first rows.
        let mut a = make_sim(64, 64);
        let mut b = make_sim(64, 64);
        set_random_start(&a, 1);
        set_random_start(&b, 1);
        // Re-seed both to the same value so RNG state is aligned before init_row.
        a.rng_state = nonzero_seed(1);
        b.rng_state = nonzero_seed(1);
        let ra = a.init_row();
        let rb = b.init_row();
        assert_eq!(ra, rb, "same seed must yield identical random row");
        assert_eq!(ra.len(), 64);
    }

    #[test]
    fn test_random_init_row_not_all_same() {
        // A random row over a reasonable width should contain both states.
        let mut sim = make_sim(128, 128);
        set_random_start(&sim, 1);
        sim.rng_state = nonzero_seed(12345);
        let row = sim.init_row();
        assert!(row.iter().any(|&c| c), "has at least one live cell");
        assert!(row.iter().any(|&c| !c), "has at least one dead cell");
    }

    #[test]
    fn test_init_row_empty_width() {
        let mut sim = make_sim(0, 0);
        assert!(sim.init_row().is_empty());
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

    #[test]
    fn test_new_initializes_state() {
        // `new` must size cur_row to width, start at row 0, and seed cur_row via
        // init_row (single centered cell with the default config). Avoids
        // `preview()`, which constructs a DebugUI and would touch js-sys on host.
        let sim = make_sim(50, 40);
        assert_eq!(sim.width, 50);
        assert_eq!(sim.height, 40);
        assert_eq!(sim.cur_row.len(), 50);
        assert_eq!(sim.y, 0);
        assert_eq!(sim.cur_row.iter().filter(|&&c| c).count(), 1);
        assert!(sim.cur_row[25]);
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
    fn test_on_canvas_resize_resets_state() {
        let mut sim = make_sim(10, 10);
        sim.y = 5;
        sim.on_canvas_resize(20, 30);
        assert_eq!(sim.width, 20);
        assert_eq!(sim.height, 30);
        assert_eq!(sim.y, 0);
        assert_eq!(sim.cur_row.len(), 20);
    }
}
