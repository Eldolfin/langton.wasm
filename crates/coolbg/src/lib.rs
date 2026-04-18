use std::{cell::RefCell, rc::Rc};

use canvas::Canvas;
use debug_ui::{DebugUI, Param, ParamParam};
use engine::{RenderConfig, Simulation, SimulationRunner, SpeedConfig};
use wasm_bindgen::prelude::*;

struct AnimationEntry {
    id: &'static str,
    name: &'static str,
}

const ANIMATIONS: &[AnimationEntry] = &[
    AnimationEntry {
        id: "langton",
        name: "Langton's Ant",
    },
    AnimationEntry {
        id: "blinker",
        name: "Blinker",
    },
];

#[wasm_bindgen]
pub fn get_animations() -> String {
    let entries: Vec<String> = ANIMATIONS
        .iter()
        .map(|a| format!(r#"{{"id":"{}","name":"{}"}}"#, a.id, a.name))
        .collect();
    format!("[{}]", entries.join(","))
}

#[wasm_bindgen]
pub async fn start_animation(id: &str) {
    console_error_panic_hook::set_once();
    match id {
        "langton" => start_langton().await,
        "blinker" => start_blinker().await,
        _ => web_sys::console::error_1(&format!("Unknown animation: {id}").into()),
    }
}

#[wasm_bindgen]
pub async fn start_preview(id: &str, canvas_element: web_sys::HtmlCanvasElement) {
    console_error_panic_hook::set_once();
    match id {
        "langton" => run_preview_langton(canvas_element).await,
        "blinker" => run_preview_blinker(canvas_element).await,
        _ => {}
    }
}

const LANGTON_PRESETS: &[(&str, &str)] = &[
    ("Many small ants", "alpha_retention=235&cell_size=5&final_speed=0.5&number_of_ants=400&speedup_frames=0&start_x=0.5&start_y=0.5"),
    ("3 trailing ants", "alpha_retention=255&final_speed=30&number_of_ants=3&speedup_frames=300&start_x=0.5&start_y=0.5&cell_size=4"),
    ("Angry ant", "alpha_retention=220&final_speed=200&number_of_ants=1&speedup_frames=0"),
    ("Flies", "alpha_retention=0&ant_color_brightness=0.3&ant_color_saturation=0&cell_border_size=0&cell_size=6&final_speed=1&number_of_ants=500&speedup_frames=120&start_x=0.5&start_y=0.5&white_color_blue=0&white_color_green=0&white_color_red=0"),
    ("Chaos", "alpha_retention=255&final_speed=40&number_of_ants=300&speedup_frames=600&start_x=0.5&start_y=0.5"),
    ("Small grid", "alpha_retention=254&ant_color_brightness=0.65&ant_color_saturation=1&cell_border_size=0&cell_size=5&final_speed=25&number_of_ants=4&speedup_frames=1200&start_x=0.5&start_y=0.5&white_color_blue=227&white_color_green=227&white_color_red=227"),
    ("1px grid", "alpha_retention=255&ant_color_brightness=0&ant_color_saturation=0.5&cell_border_size=0&cell_size=1&final_speed=5000&number_of_ants=1&speedup_frames=0&white_color_blue=255&white_color_green=255&white_color_red=255"),
];

async fn start_langton() {
    let mut debug_ui = DebugUI::new("Langton's ant parameters");
    debug_ui.presets(LANGTON_PRESETS);
    let game_config = langton::GameConfig::new(&mut debug_ui);
    let cell_size = Rc::new(RefCell::new(game_config.cell_size.clone()));
    let cell_border_size = Rc::new(RefCell::new(game_config.cell_border_size.clone()));

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
    let speed_ease_in_power = debug_ui.param(ParamParam {
        name: "speed ease-in power",
        default_value: 2.5,
        range: 1.0..=10.0,
        step_size: 0.1,
        ..Default::default()
    });
    let alpha_retention_factor = debug_ui.param(ParamParam {
        name: "alpha retention",
        default_value: 251,
        range: 0..=255,
        ..Default::default()
    });

    debug_ui.link(
        "About this animation",
        "https://codeberg.org/eldolfin/langton.wasm",
    );

    let config = Rc::new(RefCell::new(game_config));
    let step_counter = Rc::new(RefCell::new(debug_ui.step_counter()));
    let debug_ui = Rc::new(RefCell::new(debug_ui));
    let mut canvas = Canvas::new(cell_border_size.clone(), cell_size.clone());
    let needs_clear = debug_ui.borrow().needs_clear();

    loop {
        {
            let c = config.borrow();
            let bg_color = canvas::Color::Rgb {
                r: c.white_color_r.get(),
                g: c.white_color_g.get(),
                b: c.white_color_b.get(),
            };
            canvas.clear(bg_color);
        }

        step_counter.borrow_mut().reset();
        let debug_ui_ref = debug_ui.clone();
        let should_restart = Box::new(move || debug_ui_ref.borrow_mut().should_restart());

        let game = langton::Game::new(config.clone(), canvas.width(), canvas.height());
        let speed_config = SpeedConfig {
            final_steps_per_frame: final_steps_per_frame.clone(),
            speedup_frames: speedup_frames.clone(),
            speed_ease_in_power: speed_ease_in_power.clone(),
        };
        let render_config = RenderConfig {
            alpha_retention_factor: alpha_retention_factor.clone(),
        };
        let runner = SimulationRunner::new(
            game,
            speed_config,
            render_config,
            needs_clear.clone(),
            step_counter.clone(),
        );
        runner.run(&mut canvas, should_restart).await;
    }
}

async fn start_blinker() {
    let mut debug_ui = DebugUI::new("Blinker parameters");

    let cell_size: Param<usize> = debug_ui.param(ParamParam {
        name: "cell size",
        default_value: 20,
        range: 1..=50,
        ..Default::default()
    });
    let cell_border_size: Param<usize> = debug_ui.param(ParamParam {
        name: "cell border size",
        default_value: 0,
        range: 0..=5,
        ..Default::default()
    });
    let final_steps_per_frame = debug_ui.param(ParamParam {
        name: "final speed",
        default_value: 1.0,
        range: 0.0..=100.0,
        ..Default::default()
    });
    let alpha_retention_factor = debug_ui.param(ParamParam {
        name: "alpha retention",
        default_value: 255,
        range: 0..=255,
        ..Default::default()
    });

    let cell_size = Rc::new(RefCell::new(cell_size));
    let cell_border_size = Rc::new(RefCell::new(cell_border_size));
    let step_counter = Rc::new(RefCell::new(debug_ui.step_counter()));
    let debug_ui = Rc::new(RefCell::new(debug_ui));
    let mut canvas = Canvas::new(cell_border_size, cell_size);
    let needs_clear = debug_ui.borrow().needs_clear();

    loop {
        canvas.clear(canvas::Color::Rgb {
            r: 30,
            g: 30,
            b: 30,
        });
        step_counter.borrow_mut().reset();
        let debug_ui_ref = debug_ui.clone();
        let should_restart = Box::new(move || debug_ui_ref.borrow_mut().should_restart());

        let sim = dummy::BlinkingSim::new(canvas.width(), canvas.height());
        let speed_config = SpeedConfig {
            final_steps_per_frame: final_steps_per_frame.clone(),
            speedup_frames: Param::fixed(0),
            speed_ease_in_power: Param::fixed(1.0),
        };
        let render_config = RenderConfig {
            alpha_retention_factor: alpha_retention_factor.clone(),
        };
        let runner = SimulationRunner::new(
            sim,
            speed_config,
            render_config,
            needs_clear.clone(),
            step_counter.clone(),
        );
        runner.run(&mut canvas, should_restart).await;
    }
}

async fn run_preview_langton(canvas_element: web_sys::HtmlCanvasElement) {
    let cell_size: Param<usize> = Param::fixed(1);
    let cell_border_size: Param<usize> = Param::fixed(0);
    let cell_size = Rc::new(RefCell::new(cell_size));
    let cell_border_size = Rc::new(RefCell::new(cell_border_size));
    let mut canvas = Canvas::new_with_element(canvas_element, cell_border_size, cell_size);
    canvas.clear(canvas::Color::Rgb {
        r: 30,
        g: 30,
        b: 30,
    });

    let needs_clear = Rc::new(RefCell::new(false));
    let step_counter = Rc::new(RefCell::new(debug_ui::StepCounter::disabled()));
    let sim = langton::Game::preview(canvas.width(), canvas.height());
    let speed_config = SpeedConfig {
        final_steps_per_frame: Param::fixed(50.0),
        speedup_frames: Param::fixed(0),
        speed_ease_in_power: Param::fixed(1.0),
    };
    let render_config = RenderConfig {
        alpha_retention_factor: Param::fixed(251),
    };
    let runner = SimulationRunner::new(sim, speed_config, render_config, needs_clear, step_counter);
    runner.run(&mut canvas, Box::new(|| false)).await;
}

async fn run_preview_blinker(canvas_element: web_sys::HtmlCanvasElement) {
    let cell_size: Param<usize> = Param::fixed(10);
    let cell_border_size: Param<usize> = Param::fixed(0);
    let cell_size = Rc::new(RefCell::new(cell_size));
    let cell_border_size = Rc::new(RefCell::new(cell_border_size));
    let mut canvas = Canvas::new_with_element(canvas_element, cell_border_size, cell_size);
    canvas.clear(canvas::Color::Rgb {
        r: 30,
        g: 30,
        b: 30,
    });

    let needs_clear = Rc::new(RefCell::new(false));
    let step_counter = Rc::new(RefCell::new(debug_ui::StepCounter::disabled()));
    let sim = dummy::BlinkingSim::preview(canvas.width(), canvas.height());
    let speed_config = SpeedConfig {
        final_steps_per_frame: Param::fixed(1.0),
        speedup_frames: Param::fixed(0),
        speed_ease_in_power: Param::fixed(1.0),
    };
    let render_config = RenderConfig {
        alpha_retention_factor: Param::fixed(255),
    };
    let runner = SimulationRunner::new(sim, speed_config, render_config, needs_clear, step_counter);
    runner.run(&mut canvas, Box::new(|| false)).await;
}
