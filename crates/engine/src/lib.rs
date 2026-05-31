use std::{cell::RefCell, rc::Rc};

use canvas::{Canvas, Color};
use debug_ui::{Param, StepCounter};

/// Core simulation trait. Object-safe: no associated functions returning Self.
pub trait Simulation {
    /// Execute one simulation step. Called N times per frame based on speed config.
    fn step(&mut self, canvas: &mut Canvas);

    /// Called when canvas dimensions change. Sim should adapt its state.
    fn on_canvas_resize(&mut self, new_width: usize, new_height: usize);

    /// Called when clear is requested. Sim should reset its internal state.
    fn on_clear(&mut self, canvas: &mut Canvas);

    /// Background color for this simulation.
    fn bg_color(&self) -> Color;
}

pub struct SpeedConfig {
    pub final_steps_per_frame: Param<f64>,
    pub speedup_frames: Param<usize>,
    pub speed_ease_in_power: Param<f64>,
}

pub struct RenderConfig {
    pub alpha_retention_factor: Param<u8>,
}

pub struct SimulationRunner<S: Simulation> {
    sim: S,
    speed_config: SpeedConfig,
    render_config: RenderConfig,
    needs_clear: Rc<RefCell<bool>>,
    step_counter: Rc<RefCell<StepCounter>>,
    frame_counter: u64,
    step_accumulator: f64,
}

impl<S: Simulation> SimulationRunner<S> {
    pub fn new(
        sim: S,
        speed_config: SpeedConfig,
        render_config: RenderConfig,
        needs_clear: Rc<RefCell<bool>>,
        step_counter: Rc<RefCell<StepCounter>>,
    ) -> Self {
        Self {
            sim,
            speed_config,
            render_config,
            needs_clear,
            step_counter,
            frame_counter: 0,
            step_accumulator: 0.0,
        }
    }

    pub async fn run(mut self, canvas: &mut Canvas, should_stop: Box<dyn Fn() -> bool>) {
        // Tuple is (height, width) to match canvas API order; on_canvas_resize takes (width, height).
        let mut prev_canvas_size = (canvas.height(), canvas.width());
        // common::get_canvas_parent().unwrap().set_attribute("style", format!("background-cololll"));
        let style = common::get_canvas_parent().unwrap().style();
        style
            .set_property("background-color", &self.sim.bg_color().to_css_color())
            .unwrap();
        self.sim.on_clear(canvas);

        let animation = move |canvas: &mut Canvas| {
            if *self.needs_clear.borrow() {
                let style = common::get_canvas_parent().unwrap().style();
                style
                    .set_property("background-color", &self.sim.bg_color().to_css_color())
                    .unwrap();
                self.sim.on_clear(canvas);
                *self.needs_clear.borrow_mut() = false;
            }

            self.frame_counter += 1;
            let speedup = self.speed_config.speedup_frames.get() as f64;
            let ratio = (self.frame_counter as f64 / speedup).clamp(0.0, 1.0);
            let ratio = shit_ease_in(ratio, self.speed_config.speed_ease_in_power.get());
            let step = self.speed_config.final_steps_per_frame.get() * ratio;
            self.step_accumulator += step;

            let canvas_size = (canvas.height(), canvas.width());
            if canvas_size != prev_canvas_size {
                prev_canvas_size = canvas_size;
                // .1 = width, .0 = height (tuple order is (height, width))
                self.sim.on_canvas_resize(canvas_size.1, canvas_size.0);
            }

            let mut steps_this_frame: u64 = 0;
            while self.step_accumulator >= 1.0 {
                self.step_accumulator -= 1.0;
                steps_this_frame += 1;
                self.sim.step(canvas);
            }

            self.step_counter.borrow_mut().add_steps(steps_this_frame);
            canvas.fill_canvas(self.render_config.alpha_retention_factor.get());

            should_stop()
        };
        canvas.play_animation(animation).await;
    }
}

/// An ease-in felt satisfying enough by trial and error
pub fn shit_ease_in(inp: f64, power: f64) -> f64 {
    let out = inp.powf(power);
    (out + 0.005).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::shit_ease_in;
    use rstest::rstest;

    #[rstest]
    #[case(0.0, 2.0, 0.005)]
    #[case(1.0, 2.0, 1.0)]
    #[case(0.5, 1.0, 0.505)]
    #[case(0.0, 0.0, 1.0)]
    #[case(0.5, 2.0, 0.255)]
    #[case(2.0, 2.0, 1.0)]
    fn test_shit_ease_in(#[case] inp: f64, #[case] power: f64, #[case] expected: f64) {
        let result = shit_ease_in(inp, power);
        assert!(
            (result - expected).abs() < 1e-9,
            "shit_ease_in({inp}, {power}) = {result}, expected {expected}"
        );
    }

    #[test]
    fn test_shit_ease_in_always_clamped() {
        for power in [0.5, 1.0, 2.0, 5.0, 10.0] {
            for i in 0..=100 {
                let inp = i as f64 / 100.0;
                let result = shit_ease_in(inp, power);
                assert!(
                    (0.0..=1.0).contains(&result),
                    "shit_ease_in({inp}, {power}) = {result} out of [0, 1]"
                );
            }
        }
    }

    #[test]
    fn test_shit_ease_in_negative_input() {
        let result = shit_ease_in(-1.0, 2.0);
        assert!(
            (0.0..=1.0).contains(&result),
            "negative input must still clamp to [0, 1], got {result}"
        );
    }
}
