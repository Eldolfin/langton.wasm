use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{DebugColor, DebugUI, Param};
use engine::Simulation;
use engine_macros::SimulationConfig;

#[derive(SimulationConfig)]
pub struct CubeConfig {
    #[param(
        section = "Cube",
        name = "cube size",
        default = "0.4",
        range = "0.05..=0.9",
        step = 0.01
    )]
    pub cube_size: Param<f32>,
    #[param(name = "rotation axis", default = "6", range = "0..=6")]
    pub rotation_axis: Param<usize>,
    #[param(
        name = "rotation speed",
        default = "2.0",
        range = "0.1..=20.0",
        step = 0.1
    )]
    pub rotation_speed: Param<f32>,
    #[param(
        name = "perspective",
        default = "4.0",
        range = "1.5..=20.0",
        step = 0.1
    )]
    pub perspective: Param<f32>,
    #[param(
        section = "Colors",
        name = "base hue",
        default = "0.0",
        range = "0.0..=360.0",
        step = 1.0
    )]
    pub base_hue: Param<f32>,
    #[param(
        name = "color cycle speed",
        default = "1.0",
        range = "0.0..=10.0",
        step = 0.1
    )]
    pub color_cycle_speed: Param<f32>,
    #[param(name = "saturation", default = "0.8", range = "0.0..=1.0", step = 0.01)]
    pub saturation: Param<f32>,
    #[param(name = "brightness", default = "0.7", range = "0.0..=1.0", step = 0.01)]
    pub brightness: Param<f32>,
    #[param(
        name = "face color spread",
        default = "60.0",
        range = "0.0..=360.0",
        step = 1.0
    )]
    pub face_color_spread: Param<f32>,
    #[param(
        section = "Visual",
        name = "cell size",
        default = "8",
        range = "1..=50"
    )]
    pub cell_size: Param<usize>,
    #[param(name = "cell border size", default = "0", range = "0..=5")]
    pub cell_border_size: Param<usize>,
    #[param(
        name = "background color",
        default = "DebugColor { r: 10, g: 10, b: 20 }",
        color
    )]
    pub background_color: Param<DebugColor>,
}

pub const CUBE_PRESETS: &[(&str, &str)] = &[
    (
        "Rainbow Cube",
        "cube_size=0.4&rotation_axis=6&rotation_speed=2&perspective=4&base_hue=0&color_cycle_speed=2&saturation=0.85&brightness=0.7&face_color_spread=60&cell_size=6&cell_border_size=0&alpha_retention=220&final_speed=1&background_color=%230A0A14",
    ),
    (
        "Slow Spin",
        "cube_size=0.5&rotation_axis=1&rotation_speed=1&perspective=5&base_hue=200&color_cycle_speed=0.3&saturation=0.6&brightness=0.65&face_color_spread=40&cell_size=10&cell_border_size=0&alpha_retention=255&final_speed=1&background_color=%23141414",
    ),
    (
        "Neon Trails",
        "alpha_retention=252&background_color=%23050510&base_hue=280&brightness=0.8&cell_border_size=0&cell_size=4&color_cycle_speed=5&cube_size=0.35&debug=&face_color_spread=90&final_speed=2&perspective=3&rotation_axis=6&rotation_speed=3&saturation=1",
    ),
    (
        "Pastel Tumble",
        "cube_size=0.45&rotation_axis=3&rotation_speed=1.5&perspective=6&base_hue=30&color_cycle_speed=0.5&saturation=0.45&brightness=0.85&face_color_spread=50&cell_size=12&cell_border_size=0&alpha_retention=255&final_speed=1&background_color=%23F0F0F0",
    ),
    (
        "Github",
        "alpha_retention=240&background_color=%230D1117&base_hue=10&cell_size=12&cube_size=0.45&final_speed=1&speedup_frames=0",
    ),
];

const VERTICES: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
];

const FACES: [[usize; 4]; 6] = [
    [4, 5, 6, 7],
    [1, 0, 3, 2],
    [3, 7, 6, 2],
    [0, 1, 5, 4],
    [1, 2, 6, 5],
    [0, 4, 7, 3],
];

pub struct CubeSim {
    config: Rc<RefCell<CubeConfig>>,
    width: usize,
    height: usize,
    step_count: u64,
}

impl CubeSim {
    pub fn new(config: Rc<RefCell<CubeConfig>>, width: usize, height: usize) -> Self {
        Self {
            config,
            width,
            height,
            step_count: 0,
        }
    }

    pub fn preview(width: usize, height: usize) -> Self {
        let mut debug_ui = DebugUI::headless();
        let config = CubeConfig::new(&mut debug_ui);
        Self {
            config: Rc::new(RefCell::new(config)),
            width,
            height,
            step_count: 0,
        }
    }
}

impl Simulation for CubeSim {
    fn step(&mut self, canvas: &mut Canvas) {
        self.step_count += 1;
        let config = self.config.borrow();

        let cube_size = config.cube_size.get();
        let rotation_axis = config.rotation_axis.get();
        let rotation_speed = config.rotation_speed.get();
        let perspective = config.perspective.get();
        let base_hue = config.base_hue.get();
        let color_cycle_speed = config.color_cycle_speed.get();
        let saturation = config.saturation.get();
        let brightness = config.brightness.get();
        let face_color_spread = config.face_color_spread.get();

        let angle = self.step_count as f32 * rotation_speed * 0.01;

        let uses_x = matches!(rotation_axis, 0 | 3 | 4 | 6);
        let uses_y = matches!(rotation_axis, 1 | 3 | 5 | 6);
        let uses_z = matches!(rotation_axis, 2 | 4 | 5 | 6);

        let mut rotated = [[0.0f32; 3]; 8];
        for (i, v) in VERTICES.iter().enumerate() {
            let mut rv = *v;
            if uses_x {
                rv = rotate_x(rv, angle);
            }
            if uses_y {
                rv = rotate_y(rv, angle * 0.7);
            }
            if uses_z {
                rv = rotate_z(rv, angle * 0.5);
            }
            rotated[i] = rv;
        }

        let mut visible_faces: [(usize, f32); 6] = [(0, 0.0); 6];
        let mut num_visible = 0;

        for (face_idx, face) in FACES.iter().enumerate() {
            let normal = face_normal(&rotated, face);
            if normal[2] < 0.0 {
                let avg_z = face.iter().map(|&vi| rotated[vi][2]).sum::<f32>() / 4.0;
                visible_faces[num_visible] = (face_idx, avg_z);
                num_visible += 1;
            }
        }

        let visible = &mut visible_faces[..num_visible];
        visible.sort_by(|a, b| b.1.total_cmp(&a.1));

        let cx = self.width as f32 / 2.0;
        let cy = self.height as f32 / 2.0;
        let scale = cube_size * cx.min(cy);

        let current_hue = (base_hue + self.step_count as f32 * color_cycle_speed) % 360.0;

        let light_dir = normalize([0.3, 0.5, -0.8]);

        for &(face_idx, _) in visible.iter() {
            let face = &FACES[face_idx];
            let face_hue = (current_hue + face_idx as f32 * face_color_spread) % 360.0;
            let base_color = hue_to_rgb(face_hue, saturation, brightness);

            let normal = face_normal(&rotated, face);
            let n = normalize(normal);
            let dot = -(n[0] * light_dir[0] + n[1] * light_dir[1] + n[2] * light_dir[2]);
            let shade = dot.clamp(0.3, 1.0);

            let shaded = shade_color(base_color, shade);

            let mut projected = [(0.0f32, 0.0f32); 4];
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;

            for (i, &vi) in face.iter().enumerate() {
                let (px, py) = project(rotated[vi], perspective);
                let sx = cx + px * scale;
                let sy = cy - py * scale;
                projected[i] = (sx, sy);
                min_x = min_x.min(sx);
                max_x = max_x.max(sx);
                min_y = min_y.min(sy);
                max_y = max_y.max(sy);
            }

            let start_x = (min_x.floor() as isize).max(0) as usize;
            let end_x = (max_x.ceil() as usize).min(self.width.saturating_sub(1));
            let start_y = (min_y.floor() as isize).max(0) as usize;
            let end_y = (max_y.ceil() as usize).min(self.height.saturating_sub(1));

            for x in start_x..=end_x {
                for y in start_y..=end_y {
                    if point_in_quad(x as f32 + 0.5, y as f32 + 0.5, &projected) {
                        canvas.fill_rect(x, y, shaded);
                    }
                }
            }
        }
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
        self.step_count = 0;
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

fn rotate_x(v: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    [v[0], v[1] * c - v[2] * s, v[1] * s + v[2] * c]
}

fn rotate_y(v: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    [v[0] * c + v[2] * s, v[1], -v[0] * s + v[2] * c]
}

fn rotate_z(v: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]]
}

fn face_normal(rotated: &[[f32; 3]; 8], face: &[usize; 4]) -> [f32; 3] {
    let v0 = rotated[face[0]];
    let v1 = rotated[face[1]];
    let v2 = rotated[face[2]];
    let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
    [
        edge1[1] * edge2[2] - edge1[2] * edge2[1],
        edge1[2] * edge2[0] - edge1[0] * edge2[2],
        edge1[0] * edge2[1] - edge1[1] * edge2[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn project(v: [f32; 3], perspective: f32) -> (f32, f32) {
    let scale = perspective / (v[2] + perspective);
    (v[0] * scale, v[1] * scale)
}

fn point_in_quad(px: f32, py: f32, quad: &[(f32, f32); 4]) -> bool {
    let mut sign = None;
    for i in 0..4 {
        let j = (i + 1) % 4;
        let cross =
            (quad[j].0 - quad[i].0) * (py - quad[i].1) - (quad[j].1 - quad[i].1) * (px - quad[i].0);
        let s = cross >= 0.0;
        match sign {
            None => sign = Some(s),
            Some(prev) => {
                if prev != s {
                    return false;
                }
            }
        }
    }
    true
}

fn shade_color(color: Color, shade: f32) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * shade) as u8,
            g: (g as f32 * shade) as u8,
            b: (b as f32 * shade) as u8,
        },
        other => other,
    }
}

fn hue_to_rgb(hue: f32, saturation: f32, lightness: f32) -> Color {
    let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let h_prime = hue / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let m = lightness - c / 2.0;

    let (r, g, b) = if (0.0..1.0).contains(&h_prime) {
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

    Color::Rgb {
        r: ((r + m) * 255.0).round() as u8,
        g: ((g + m) * 255.0).round() as u8,
        b: ((b + m) * 255.0).round() as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotate_x_identity() {
        let v = [1.0, 0.0, 0.0];
        let result = rotate_x(v, 0.0);
        assert!((result[0] - 1.0).abs() < 1e-6);
        assert!(result[1].abs() < 1e-6);
        assert!(result[2].abs() < 1e-6);
    }

    #[test]
    fn test_rotate_y_quarter_turn() {
        let v = [1.0, 0.0, 0.0];
        let result = rotate_y(v, std::f32::consts::FRAC_PI_2);
        assert!(result[0].abs() < 1e-5);
        assert!(result[1].abs() < 1e-5);
        assert!((result[2] + 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_normalize() {
        let v = [3.0, 4.0, 0.0];
        let n = normalize(v);
        assert!((n[0] - 0.6).abs() < 1e-6);
        assert!((n[1] - 0.8).abs() < 1e-6);
        assert!(n[2].abs() < 1e-6);
    }

    #[test]
    fn test_normalize_zero() {
        let n = normalize([0.0, 0.0, 0.0]);
        assert_eq!(n, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_point_in_quad_inside() {
        let quad = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_quad(5.0, 5.0, &quad));
    }

    #[test]
    fn test_point_in_quad_outside() {
        let quad = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(!point_in_quad(15.0, 5.0, &quad));
    }

    #[test]
    fn test_point_in_quad_edge() {
        let quad = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_quad(0.0, 5.0, &quad));
    }

    #[test]
    fn test_project_center() {
        let (x, y) = project([0.0, 0.0, 0.0], 4.0);
        assert!(x.abs() < 1e-6);
        assert!(y.abs() < 1e-6);
    }

    #[test]
    fn test_project_perspective() {
        let (x, _) = project([2.0, 0.0, 0.0], 4.0);
        assert!((x - 2.0).abs() < 1e-6);
        let (x, _) = project([2.0, 0.0, 4.0], 4.0);
        assert!((x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_shade_color() {
        let color = Color::Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        let shaded = shade_color(color, 0.5);
        assert_eq!(
            shaded,
            Color::Rgb {
                r: 100,
                g: 50,
                b: 25
            }
        );
    }

    #[test]
    fn test_hue_to_rgb_red() {
        let color = hue_to_rgb(0.0, 1.0, 0.5);
        match color {
            Color::Rgb { r, g, b } => {
                assert_eq!(r, 255);
                assert_eq!(g, 0);
                assert_eq!(b, 0);
            }
            _ => panic!("expected Rgb"),
        }
    }

    #[test]
    fn test_face_normal_front() {
        let normal = face_normal(&VERTICES, &FACES[0]);
        assert!(normal[2] > 0.0);
    }

    #[test]
    fn test_face_normal_back() {
        let normal = face_normal(&VERTICES, &FACES[1]);
        assert!(normal[2] < 0.0);
    }

    #[test]
    fn test_cube_sim_resize() {
        let mut debug_ui = DebugUI::headless();
        let config = CubeConfig::new(&mut debug_ui);
        let mut sim = CubeSim::new(Rc::new(RefCell::new(config)), 100, 100);
        assert_eq!(sim.width, 100);
        assert_eq!(sim.height, 100);
        sim.on_canvas_resize(200, 150);
        assert_eq!(sim.width, 200);
        assert_eq!(sim.height, 150);
    }
}
