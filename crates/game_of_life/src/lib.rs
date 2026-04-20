use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugColor, DebugUI, Param};
use engine::Simulation;
use engine_macros::SimulationConfig;

#[derive(SimulationConfig)]
pub struct GameOfLifeConfig {
    #[param(
        section = "Rules",
        name = "birth rule",
        default = "8",
        range = "0..=511"
    )]
    pub birth_rule: Param<usize>,
    #[param(name = "survival rule", default = "12", range = "0..=511")]
    pub survival_rule: Param<usize>,
    #[param(
        section = "Visual",
        name = "cell size",
        default = "5",
        range = "1..=50"
    )]
    pub cell_size: Param<usize>,
    #[param(name = "cell border size", default = "0", range = "0..=5")]
    pub cell_border_size: Param<usize>,
    #[param(
        name = "alive color",
        default = "DebugColor { r: 0, g: 255, b: 102 }",
        color
    )]
    pub alive_color: Param<DebugColor>,
    #[param(
        name = "dead color",
        default = "DebugColor { r: 30, g: 30, b: 30 }",
        color
    )]
    pub dead_color: Param<DebugColor>,
    #[param(
        section = "Initial State",
        name = "initial density",
        default = "0.3",
        range = "0.0..=1.0",
        step = 0.01,
        needs_restart
    )]
    pub initial_density: Param<f32>,
    #[param(
        name = "seed",
        default = "42",
        range = "0..=4294967295",
        needs_restart
    )]
    pub seed: Param<u32>,
}

// Bitmask encoding: bit N set means N neighbors triggers the rule.
// Classic Conway B3/S23: birth = 1<<3 = 8, survival = (1<<2)|(1<<3) = 12.
pub const GOL_PRESETS: &[(&str, &str)] = &[
    (
        "Conway (B3/S23)",
        "birth_rule=8&survival_rule=12&initial_density=0.3",
    ),
    (
        "HighLife (B36/S23)",
        "birth_rule=72&survival_rule=12&initial_density=0.2",
    ),
    (
        "Day & Night (B3678/S34678)",
        "birth_rule=456&survival_rule=472&initial_density=0.5",
    ),
    (
        "Seeds (B2/S—)",
        "birth_rule=4&survival_rule=0&initial_density=0.005&cell_size=3",
    ),
    (
        "Maze (B3/S12345)",
        "birth_rule=8&survival_rule=62&initial_density=0.005&cell_size=3",
    ),
    (
        "Replicator (B1357/S1357)",
        "birth_rule=170&survival_rule=170&initial_density=0.005&cell_size=3",
    ),
    (
        "Diamoeba (B35678/S5678)",
        "birth_rule=488&survival_rule=480&initial_density=0.5",
    ),
    (
        "Anneal (B4678/S35678)",
        "birth_rule=464&survival_rule=488&initial_density=0.5",
    ),
];

pub struct GameOfLife {
    current: Vec<bool>,
    next: Vec<bool>,
    config: Rc<RefCell<GameOfLifeConfig>>,
    width: usize,
    height: usize,
    click_queue: Rc<RefCell<Vec<(usize, usize)>>>,
    needs_full_render: bool,
}

impl GameOfLife {
    pub fn new(
        config: Rc<RefCell<GameOfLifeConfig>>,
        width: usize,
        height: usize,
        click_queue: Rc<RefCell<Vec<(usize, usize)>>>,
    ) -> Self {
        let cfg = config.borrow();
        let density = cfg.initial_density.get();
        let mut rng_state = cfg.seed.get();
        drop(cfg);
        if rng_state == 0 {
            rng_state = 1;
        }

        let total = width * height;
        let threshold = (f64::from(density) * f64::from(u32::MAX)) as u32;
        let mut current = vec![false; total];
        for cell in &mut current {
            *cell = xorshift32(&mut rng_state) < threshold;
        }

        Self {
            current,
            next: vec![false; total],
            config,
            width,
            height,
            click_queue,
            needs_full_render: true,
        }
    }

    pub fn preview(width: usize, height: usize) -> Self {
        let mut debug_ui = DebugUI::headless();
        let config = GameOfLifeConfig::new(&mut debug_ui);
        Self::new(
            Rc::new(RefCell::new(config)),
            width,
            height,
            Rc::new(RefCell::new(vec![])),
        )
    }
}

impl Simulation for GameOfLife {
    fn step(&mut self, canvas: &mut Canvas) {
        let config = self.config.borrow();
        let birth_rule = config.birth_rule.get();
        let survival_rule = config.survival_rule.get();
        let alive_c = config.alive_color.get();
        let dead_c = config.dead_color.get();
        let alive_color = Color::Rgb {
            r: alive_c.r,
            g: alive_c.g,
            b: alive_c.b,
        };
        let dead_color = Color::Rgb {
            r: dead_c.r,
            g: dead_c.g,
            b: dead_c.b,
        };
        drop(config);

        let clicks: Vec<(usize, usize)> = self.click_queue.borrow_mut().drain(..).collect();
        for (x, y) in clicks {
            if x < self.width && y < self.height {
                let idx = x * self.height + y;
                self.current[idx] = !self.current[idx];
                let color = if self.current[idx] {
                    alive_color
                } else {
                    dead_color
                };
                canvas.fill_rect(x, y, color);
            }
        }

        if self.needs_full_render {
            for x in 0..self.width {
                for y in 0..self.height {
                    if self.current[x * self.height + y] {
                        canvas.fill_rect(x, y, alive_color);
                    }
                }
            }
            self.needs_full_render = false;
        }

        for x in 0..self.width {
            for y in 0..self.height {
                let idx = x * self.height + y;
                let neighbors = count_neighbors(&self.current, self.width, self.height, x, y);
                let alive = self.current[idx];
                let new_alive = if alive {
                    (survival_rule >> neighbors) & 1 != 0
                } else {
                    (birth_rule >> neighbors) & 1 != 0
                };
                self.next[idx] = new_alive;
                if new_alive != alive {
                    canvas.fill_rect(
                        x,
                        y,
                        if new_alive { alive_color } else { dead_color },
                    );
                }
            }
        }

        std::mem::swap(&mut self.current, &mut self.next);
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        let mut new_current = vec![false; new_width * new_height];
        let copy_w = self.width.min(new_width);
        let copy_h = self.height.min(new_height);
        for x in 0..copy_w {
            for y in 0..copy_h {
                new_current[x * new_height + y] = self.current[x * self.height + y];
            }
        }
        self.width = new_width;
        self.height = new_height;
        self.current = new_current;
        self.next = vec![false; new_width * new_height];
        self.needs_full_render = true;
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
        self.current.fill(false);
        self.next.fill(false);
    }

    fn bg_color(&self) -> Color {
        let c = self.config.borrow();
        let bg = c.dead_color.get();
        Color::Rgb {
            r: bg.r,
            g: bg.g,
            b: bg.b,
        }
    }
}

fn count_neighbors(grid: &[bool], width: usize, height: usize, x: usize, y: usize) -> u32 {
    let mut count = 0u32;
    for dy in [-1i32, 0, 1] {
        for dx in [-1i32, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = ((x as i32 + dx).rem_euclid(width as i32)) as usize;
            let ny = ((y as i32 + dy).rem_euclid(height as i32)) as usize;
            if grid[nx * height + ny] {
                count += 1;
            }
        }
    }
    count
}

fn xorshift32(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_xorshift32_deterministic() {
        let mut s1 = 42u32;
        let mut s2 = 42u32;
        for _ in 0..50 {
            assert_eq!(xorshift32(&mut s1), xorshift32(&mut s2));
        }
    }

    #[test]
    fn test_xorshift32_different_seeds_diverge() {
        let mut s1 = 1u32;
        let mut s2 = 2u32;
        assert_ne!(xorshift32(&mut s1), xorshift32(&mut s2));
    }

    #[rstest]
    #[case(2, 2, &[], 0)]
    #[case(2, 2, &[(1, 1), (2, 1), (3, 1), (1, 2), (3, 2), (1, 3), (2, 3), (3, 3)], 8)]
    #[case(0, 0, &[(4, 4)], 1)]
    #[case(0, 0, &[(1, 0), (0, 1), (1, 1)], 3)]
    fn test_count_neighbors(
        #[case] x: usize,
        #[case] y: usize,
        #[case] alive: &[(usize, usize)],
        #[case] expected: u32,
    ) {
        let width = 5;
        let height = 5;
        let mut grid = vec![false; width * height];
        for &(ax, ay) in alive {
            grid[ax * height + ay] = true;
        }
        assert_eq!(count_neighbors(&grid, width, height, x, y), expected);
    }

    #[test]
    fn test_conway_blinker_oscillation() {
        let width = 5;
        let height = 5;
        let birth_rule: usize = 8;
        let survival_rule: usize = 12;

        let mut grid = vec![false; width * height];
        grid[2 * height + 1] = true;
        grid[2 * height + 2] = true;
        grid[2 * height + 3] = true;

        let mut next = vec![false; width * height];
        for x in 0..width {
            for y in 0..height {
                let neighbors = count_neighbors(&grid, width, height, x, y);
                let alive = grid[x * height + y];
                next[x * height + y] = if alive {
                    (survival_rule >> neighbors) & 1 != 0
                } else {
                    (birth_rule >> neighbors) & 1 != 0
                };
            }
        }

        assert!(!next[2 * height + 1]);
        assert!(next[2 * height + 2]);
        assert!(!next[2 * height + 3]);
        assert!(next[height + 2]);
        assert!(next[3 * height + 2]);
    }
}
