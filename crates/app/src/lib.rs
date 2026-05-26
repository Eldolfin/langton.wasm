use std::{cell::RefCell, rc::Rc};

use canvas::Canvas;
use debug_ui::{DebugUI, Param, ParamParam};
use engine::{RenderConfig, Simulation, SimulationRunner, SpeedConfig};
use wasm_bindgen::prelude::*;

// --- Registry -----------------------------------------------------------

type StartFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

struct AnimationEntry {
    id: &'static str,
    name: &'static str,
    start_fn: fn() -> StartFuture,
    preview_fn: fn(web_sys::HtmlCanvasElement) -> StartFuture,
}

// Thin wrapper fns bridge async fn → fn pointer (closures can't be stored in const).
fn langton_start() -> StartFuture {
    Box::pin(start_langton())
}
fn langton_preview(el: web_sys::HtmlCanvasElement) -> StartFuture {
    Box::pin(run_preview_langton(el))
}
fn blinker_start() -> StartFuture {
    Box::pin(start_blinker())
}
fn blinker_preview(el: web_sys::HtmlCanvasElement) -> StartFuture {
    Box::pin(run_preview_blinker(el))
}
fn cube_start() -> StartFuture {
    Box::pin(start_cube())
}
fn cube_preview(el: web_sys::HtmlCanvasElement) -> StartFuture {
    Box::pin(run_preview_cube(el))
}
fn sierpinski_start() -> StartFuture {
    Box::pin(start_sierpinski())
}
fn sierpinski_preview(el: web_sys::HtmlCanvasElement) -> StartFuture {
    Box::pin(run_preview_sierpinski(el))
}

/// Single source of truth: add one entry here to register a new animation.
const REGISTRY: &[AnimationEntry] = &[
    AnimationEntry {
        id: "langton",
        name: "Langton's Ant",
        start_fn: langton_start,
        preview_fn: langton_preview,
    },
    AnimationEntry {
        id: "blinker",
        name: "Blinker",
        start_fn: blinker_start,
        preview_fn: blinker_preview,
    },
    AnimationEntry {
        id: "cube",
        name: "Rotating Cube",
        start_fn: cube_start,
        preview_fn: cube_preview,
    },
    AnimationEntry {
        id: "sierpinski",
        name: "Sierpinski (Chaos Game)",
        start_fn: sierpinski_start,
        preview_fn: sierpinski_preview,
    },
];

// --- WASM exports --------------------------------------------------------

#[wasm_bindgen]
pub fn get_animations() -> String {
    fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }
    let entries: Vec<String> = REGISTRY
        .iter()
        .map(|a| {
            format!(
                r#"{{"id":"{}","name":"{}"}}"#,
                escape_json(a.id),
                escape_json(a.name)
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

#[wasm_bindgen]
pub async fn start_animation(id: &str) {
    console_error_panic_hook::set_once();
    match REGISTRY.iter().find(|e| e.id == id) {
        Some(entry) => (entry.start_fn)().await,
        None => web_sys::console::error_1(&format!("Unknown animation: {id}").into()),
    }
}

#[wasm_bindgen]
pub async fn start_preview(id: &str, canvas_element: web_sys::HtmlCanvasElement) {
    console_error_panic_hook::set_once();
    if let Some(entry) = REGISTRY.iter().find(|e| e.id == id) {
        (entry.preview_fn)(canvas_element).await;
    }
}

async fn start_langton() {
    let mut debug_ui = DebugUI::new("Langton's ant parameters");
    debug_ui.presets(langton::LANGTON_PRESETS);
    let game_config = langton::GameConfig::new(&mut debug_ui);
    let cell_size = Rc::new(RefCell::new(game_config.cell_size.clone()));
    let cell_border_size = Rc::new(RefCell::new(game_config.cell_border_size.clone()));

    debug_ui.start_section("Animation Speed");
    let final_steps_per_frame = debug_ui.param(ParamParam {
        name: "final speed",
        default_value: 0.2,
        // Upper bound 1M intentional: enables extreme benchmark scenarios (1px grid preset).
        // At these speeds the browser may stutter; that is acceptable.
        range: 0.00..=1_000_000.0,
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

    debug_ui.add_footer();

    let config = Rc::new(RefCell::new(game_config));
    let step_counter = Rc::new(RefCell::new(debug_ui.step_counter()));
    let debug_ui = Rc::new(RefCell::new(debug_ui));
    let mut canvas = Canvas::new(cell_border_size.clone(), cell_size.clone());
    let needs_clear = debug_ui.borrow().needs_clear();

    loop {
        {
            let c = config.borrow();
            let bg = c.common_cell_color.get();
            let bg_color = canvas::Color::Rgb {
                r: bg.r,
                g: bg.g,
                b: bg.b,
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
    debug_ui.add_footer();

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

async fn run_preview<S: Simulation>(
    canvas_element: web_sys::HtmlCanvasElement,
    sim: S,
    cell_size: usize,
    final_steps_per_frame: f64,
    alpha_retention: u8,
) {
    let cell_size = Rc::new(RefCell::new(Param::fixed(cell_size)));
    let cell_border_size = Rc::new(RefCell::new(Param::fixed(0usize)));
    let mut canvas = Canvas::new_with_element(canvas_element, cell_border_size, cell_size);
    canvas.clear(sim.bg_color());

    let needs_clear = Rc::new(RefCell::new(false));
    let step_counter = Rc::new(RefCell::new(debug_ui::StepCounter::disabled()));
    let speed_config = SpeedConfig {
        final_steps_per_frame: Param::fixed(final_steps_per_frame),
        speedup_frames: Param::fixed(0usize),
        speed_ease_in_power: Param::fixed(1.0f64),
    };
    let render_config = RenderConfig {
        alpha_retention_factor: Param::fixed(alpha_retention),
    };
    let runner = SimulationRunner::new(sim, speed_config, render_config, needs_clear, step_counter);
    runner.run(&mut canvas, Box::new(|| false)).await;
}

async fn run_preview_langton(canvas_element: web_sys::HtmlCanvasElement) {
    let w = canvas_element.width() as usize;
    let h = canvas_element.height() as usize;
    run_preview(canvas_element, langton::Game::preview(w, h), 1, 50.0, 251).await;
}

async fn run_preview_blinker(canvas_element: web_sys::HtmlCanvasElement) {
    let w = canvas_element.width() as usize;
    let h = canvas_element.height() as usize;
    run_preview(canvas_element, dummy::BlinkingSim::new(w, h), 10, 1.0, 255).await;
}

async fn start_cube() {
    let mut debug_ui = DebugUI::new("Rotating Cube parameters");
    debug_ui.presets(cube::CUBE_PRESETS);
    let cube_config = cube::CubeConfig::new(&mut debug_ui);
    let cell_size = Rc::new(RefCell::new(cube_config.cell_size.clone()));
    let cell_border_size = Rc::new(RefCell::new(cube_config.cell_border_size.clone()));

    debug_ui.start_section("Animation Speed");
    let final_steps_per_frame = debug_ui.param(ParamParam {
        name: "final speed",
        default_value: 1.0,
        range: 0.01..=100.0,
        scale: debug_ui::Scale::Logarithmic,
        ..Default::default()
    });
    let speedup_frames = debug_ui.param(ParamParam {
        name: "speedup frames",
        default_value: 0,
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
        default_value: 220,
        range: 0..=255,
        ..Default::default()
    });

    debug_ui.add_footer();

    let config = Rc::new(RefCell::new(cube_config));
    let step_counter = Rc::new(RefCell::new(debug_ui.step_counter()));
    let debug_ui = Rc::new(RefCell::new(debug_ui));
    let mut canvas = Canvas::new(cell_border_size.clone(), cell_size.clone());
    let needs_clear = debug_ui.borrow().needs_clear();

    loop {
        {
            let c = config.borrow();
            let bg = c.background_color.get();
            let bg_color = canvas::Color::Rgb {
                r: bg.r,
                g: bg.g,
                b: bg.b,
            };
            canvas.clear(bg_color);
        }

        step_counter.borrow_mut().reset();
        let debug_ui_ref = debug_ui.clone();
        let should_restart = Box::new(move || debug_ui_ref.borrow_mut().should_restart());

        let sim = cube::CubeSim::new(config.clone(), canvas.width(), canvas.height());
        let speed_config = SpeedConfig {
            final_steps_per_frame: final_steps_per_frame.clone(),
            speedup_frames: speedup_frames.clone(),
            speed_ease_in_power: speed_ease_in_power.clone(),
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

async fn run_preview_cube(canvas_element: web_sys::HtmlCanvasElement) {
    let w = canvas_element.width() as usize;
    let h = canvas_element.height() as usize;
    run_preview(canvas_element, cube::CubeSim::preview(w, h), 4, 1.0, 220).await;
}

async fn start_sierpinski() {
    let mut debug_ui = DebugUI::new("Sierpinski (Chaos Game) parameters");
    debug_ui.presets(sierpinski::SIERPINSKI_PRESETS);
    let sierpinski_config = sierpinski::SierpinskiConfig::new(&mut debug_ui);
    let cell_size = Rc::new(RefCell::new(sierpinski_config.cell_size.clone()));
    let cell_border_size = Rc::new(RefCell::new(sierpinski_config.cell_border_size.clone()));

    debug_ui.start_section("Animation Speed");
    let final_steps_per_frame = debug_ui.param(ParamParam {
        name: "final speed",
        default_value: 100.0,
        range: 1.0..=10_000.0,
        scale: debug_ui::Scale::Logarithmic,
        ..Default::default()
    });
    let speedup_frames = debug_ui.param(ParamParam {
        name: "speedup frames",
        default_value: 0,
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
        default_value: 255,
        range: 0..=255,
        ..Default::default()
    });

    debug_ui.add_footer();

    let config = Rc::new(RefCell::new(sierpinski_config));
    let step_counter = Rc::new(RefCell::new(debug_ui.step_counter()));
    let debug_ui = Rc::new(RefCell::new(debug_ui));
    let mut canvas = Canvas::new(cell_border_size.clone(), cell_size.clone());
    let needs_clear = debug_ui.borrow().needs_clear();

    loop {
        {
            let c = config.borrow();
            let bg = c.background_color.get();
            let bg_color = canvas::Color::Rgb {
                r: bg.r,
                g: bg.g,
                b: bg.b,
            };
            canvas.clear(bg_color);
        }

        step_counter.borrow_mut().reset();
        let debug_ui_ref = debug_ui.clone();
        let should_restart = Box::new(move || debug_ui_ref.borrow_mut().should_restart());

        let sim = sierpinski::SierpinskiSim::new(config.clone(), canvas.width(), canvas.height());
        let speed_config = SpeedConfig {
            final_steps_per_frame: final_steps_per_frame.clone(),
            speedup_frames: speedup_frames.clone(),
            speed_ease_in_power: speed_ease_in_power.clone(),
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

async fn run_preview_sierpinski(canvas_element: web_sys::HtmlCanvasElement) {
    let w = canvas_element.width() as usize;
    let h = canvas_element.height() as usize;
    run_preview(
        canvas_element,
        sierpinski::SierpinskiSim::preview(w, h),
        1,
        80.0,
        255,
    )
    .await;
}
