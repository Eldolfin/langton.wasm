use debug_ui::Config;

#[derive(Config)]
pub struct GameConfig {
    /// doc shown on hover
    #[field(default = 0.05, range = 0.0..1000.0, scale = debug_ui::Scale::Logarithmic)]
    initial_steps_per_frame: f64,
    speedup_frames: f64,
    start_x_rel: f32,
    start_y_rel: f32,
}

fn main() {
    GameConfig::debug_ui_config();
}
