use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugColor, DebugUI, Param};
use engine::Simulation;
use engine_macros::SimulationConfig;

#[derive(SimulationConfig)]
pub struct SierpinskiConfig {
    #[param(
        section = "Chaos game",
        name = "vertices",
        default = "3",
        range = "3..=8",
        needs_restart
    )]
    pub num_vertices: Param<usize>,
    #[param(
        name = "jump ratio",
        default = "0.5",
        range = "0.1..=0.9",
        step = 0.01,
        needs_restart
    )]
    pub jump_ratio: Param<f32>,
    #[param(
        section = "Visual",
        name = "cell size",
        default = "2",
        range = "1..=20"
    )]
    pub cell_size: Param<usize>,
    #[param(name = "cell border size", default = "0", range = "0..=5")]
    pub cell_border_size: Param<usize>,
    #[param(
        name = "point color",
        default = "DebugColor { r: 240, g: 240, b: 240 }",
        color
    )]
    pub point_color: Param<DebugColor>,
    #[param(
        name = "background color",
        default = "DebugColor { r: 12, g: 12, b: 24 }",
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

pub const SIERPINSKI_PRESETS: &[(&str, &str)] = &[
    (
        "Classic triangle",
        "vertices=3&jump_ratio=0.5&cell_size=2&cell_border_size=0&point_color=%23F0F0F0&background_color=%230C0C18&seed=1&final_speed=50&speedup_frames=0&alpha_retention=255",
    ),
    (
        "Sierpinski pentagon",
        "vertices=5&jump_ratio=0.5&cell_size=2&cell_border_size=0&point_color=%23FFC8A0&background_color=%23101020&seed=1&final_speed=80&speedup_frames=0&alpha_retention=255",
    ),
    (
        "Hexagon swirl",
        "vertices=6&jump_ratio=0.38&cell_size=2&cell_border_size=0&point_color=%23A0E0FF&background_color=%23080814&seed=1&final_speed=80&speedup_frames=0&alpha_retention=255",
    ),
    (
        "Fading drift",
        "vertices=4&jump_ratio=0.45&cell_size=2&cell_border_size=0&point_color=%23FFFFFF&background_color=%23000000&seed=42&final_speed=200&speedup_frames=300&alpha_retention=240",
    ),
];

pub struct SierpinskiSim {
    config: Rc<RefCell<SierpinskiConfig>>,
    width: usize,
    height: usize,
    point_x: f32,
    point_y: f32,
    rng_state: u32,
}

impl SierpinskiSim {
    pub fn new(config: Rc<RefCell<SierpinskiConfig>>, width: usize, height: usize) -> Self {
        let seed = config.borrow().seed.get();
        let mut sim = Self {
            config,
            width,
            height,
            point_x: 0.0,
            point_y: 0.0,
            rng_state: nonzero_seed(seed),
        };
        sim.recenter_point();
        sim
    }

    pub fn preview(width: usize, height: usize) -> Self {
        let mut debug_ui = DebugUI::headless();
        let config = SierpinskiConfig::new(&mut debug_ui);
        Self::new(Rc::new(RefCell::new(config)), width, height)
    }

    fn recenter_point(&mut self) {
        self.point_x = self.width as f32 / 2.0;
        self.point_y = self.height as f32 / 2.0;
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

    fn vertex(&self, idx: usize, n: usize) -> (f32, f32) {
        let cx = self.width as f32 / 2.0;
        let cy = self.height as f32 / 2.0;
        // 0.92 leaves a thin margin so the outermost points stay inside the grid.
        let r = cx.min(cy) * 0.92;
        let theta = (idx as f32 / n as f32) * std::f32::consts::TAU
            - std::f32::consts::FRAC_PI_2;
        let (sin_t, cos_t) = theta.sin_cos();
        (cx + r * cos_t, cy + r * sin_t)
    }

    fn advance(&mut self) {
        let (n, ratio) = {
            let cfg = self.config.borrow();
            (cfg.num_vertices.get().max(3), cfg.jump_ratio.get())
        };
        let idx = (self.next_rand() as usize) % n;
        let (vx, vy) = self.vertex(idx, n);
        self.point_x += (vx - self.point_x) * ratio;
        self.point_y += (vy - self.point_y) * ratio;
    }
}

impl Simulation for SierpinskiSim {
    fn step(&mut self, canvas: &mut Canvas) {
        self.advance();
        let pt = self.config.borrow().point_color.get();

        let px = self.point_x as usize;
        let py = self.point_y as usize;
        if px < self.width && py < self.height {
            canvas.fill_rect(
                px,
                py,
                Color::Rgb {
                    r: pt.r,
                    g: pt.g,
                    b: pt.b,
                },
            );
        }
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.recenter_point();
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
        let seed = self.config.borrow().seed.get();
        self.rng_state = nonzero_seed(seed);
        self.recenter_point();
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

    fn make_sim(width: usize, height: usize) -> SierpinskiSim {
        let mut debug_ui = DebugUI::headless();
        let config = SierpinskiConfig::new(&mut debug_ui);
        SierpinskiSim::new(Rc::new(RefCell::new(config)), width, height)
    }

    #[test]
    fn test_initial_point_centered() {
        let sim = make_sim(100, 80);
        assert!((sim.point_x - 50.0).abs() < 1e-6);
        assert!((sim.point_y - 40.0).abs() < 1e-6);
    }

    #[test]
    fn test_on_canvas_resize_recenters() {
        let mut sim = make_sim(100, 100);
        sim.point_x = 7.0;
        sim.point_y = 9.0;
        sim.on_canvas_resize(40, 60);
        assert_eq!(sim.width, 40);
        assert_eq!(sim.height, 60);
        assert!((sim.point_x - 20.0).abs() < 1e-6);
        assert!((sim.point_y - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_vertex_zero_is_top() {
        let sim = make_sim(100, 100);
        let (vx, vy) = sim.vertex(0, 3);
        // First vertex sits at the top (theta = -pi/2): x at center, y above center.
        assert!((vx - 50.0).abs() < 1e-3);
        assert!(vy < 50.0);
    }

    #[rstest]
    #[case(3)]
    #[case(4)]
    #[case(5)]
    #[case(6)]
    #[case(7)]
    #[case(8)]
    fn test_vertices_lie_on_circle(#[case] n: usize) {
        let sim = make_sim(200, 200);
        let cx = 100.0_f32;
        let cy = 100.0_f32;
        let r = 100.0_f32 * 0.92;
        for i in 0..n {
            let (vx, vy) = sim.vertex(i, n);
            let dist = ((vx - cx).powi(2) + (vy - cy).powi(2)).sqrt();
            assert!(
                (dist - r).abs() < 1e-2,
                "vertex {i}/{n} dist {dist} != r {r}"
            );
        }
    }

    #[test]
    fn test_rng_is_deterministic() {
        let mut a = make_sim(100, 100);
        let mut b = make_sim(100, 100);
        for _ in 0..10 {
            assert_eq!(a.next_rand(), b.next_rand());
        }
    }

    #[test]
    fn test_rng_never_zero_after_seeding() {
        let mut sim = make_sim(100, 100);
        // xorshift32 hits every nonzero u32; check the first few aren't zero.
        for _ in 0..1000 {
            assert_ne!(sim.next_rand(), 0);
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
    fn test_chaos_game_converges_into_triangle() {
        let mut sim = make_sim(400, 400);
        // Default config: n=3, ratio=0.5. After enough warmup, points must lie
        // inside the bounding box of the 3 chosen vertices.
        let (v0x, v0y) = sim.vertex(0, 3);
        let (v1x, v1y) = sim.vertex(1, 3);
        let (v2x, v2y) = sim.vertex(2, 3);
        let min_x = v0x.min(v1x).min(v2x) - 1.0;
        let max_x = v0x.max(v1x).max(v2x) + 1.0;
        let min_y = v0y.min(v1y).min(v2y) - 1.0;
        let max_y = v0y.max(v1y).max(v2y) + 1.0;

        // Warm up: throw away the first 50 points (still converging from center).
        for _ in 0..50 {
            sim.advance();
        }
        for _ in 0..1000 {
            sim.advance();
            assert!(
                sim.point_x >= min_x
                    && sim.point_x <= max_x
                    && sim.point_y >= min_y
                    && sim.point_y <= max_y,
                "point ({}, {}) escaped triangle bbox [{}..{}, {}..{}]",
                sim.point_x,
                sim.point_y,
                min_x,
                max_x,
                min_y,
                max_y
            );
        }
    }

    #[test]
    fn test_preview_constructor() {
        let sim = SierpinskiSim::preview(50, 50);
        assert_eq!(sim.width, 50);
        assert_eq!(sim.height, 50);
        assert!((sim.point_x - 25.0).abs() < 1e-6);
        assert!((sim.point_y - 25.0).abs() < 1e-6);
    }

    #[test]
    fn test_bg_color_uses_config() {
        let sim = make_sim(20, 20);
        match sim.bg_color() {
            Color::Rgb { r, g, b } => {
                assert_eq!((r, g, b), (12, 12, 24));
            }
            other => panic!("expected Rgb, got {other:?}"),
        }
    }
}
