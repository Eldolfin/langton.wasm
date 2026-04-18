use canvas::{Canvas, Color};
use engine::Simulation;

pub struct BlinkingSim {
    x: usize,
    y: usize,
    on: bool,
}

impl BlinkingSim {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            x: width / 2,
            y: height / 2,
            on: false,
        }
    }

}

impl Simulation for BlinkingSim {
    fn step(&mut self, canvas: &mut Canvas) {
        let color = if self.on {
            Color::Rgb {
                r: 255,
                g: 255,
                b: 255,
            }
        } else {
            Color::Rgb { r: 0, g: 0, b: 0 }
        };
        canvas.fill_rect(self.x, self.y, color);
        self.on = !self.on;
    }

    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize) {
        self.x = new_width / 2;
        self.y = new_height / 2;
    }

    fn on_clear(&mut self, canvas: &mut Canvas) {
        canvas.clear(self.bg_color());
    }

    fn bg_color(&self) -> Color {
        Color::Rgb {
            r: 30,
            g: 30,
            b: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blink_toggle() {
        let sim = BlinkingSim::new(10, 10);
        assert!(!sim.on);
    }

    #[test]
    fn test_on_canvas_resize() {
        let mut sim = BlinkingSim::new(10, 10);
        assert_eq!(sim.x, 5);
        assert_eq!(sim.y, 5);
        sim.on_canvas_resize(20, 30);
        assert_eq!(sim.x, 10);
        assert_eq!(sim.y, 15);
    }

    #[test]
    fn test_initial_dimensions() {
        let sim = BlinkingSim::new(40, 60);
        assert_eq!(sim.x, 20);
        assert_eq!(sim.y, 30);
    }
}
