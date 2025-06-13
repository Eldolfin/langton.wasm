// // Example usage
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
// #[wasm_bindgen(start)]
// async fn start() {
//     console_error_panic_hook::set_once();
//     let canvas = Canvas::get_element_by_id("canvas")
//         .unwrap()
//         .with_cell_size(10.);
//     const N_FRAMES: usize = 100;

//     // shared state across frames
//     let mut frame_counter = 0;
//     let animation = move |canvas: &mut Canvas| {
//         for x in 0..canvas.width() {
//             for y in 0..canvas.height() {
//                 let color = if (x + y) % 2 == (frame_counter / 60) % 2 {
//                     let x = x as f32;
//                     let y = y as f32;
//                     let h = canvas.height() as f32;
//                     let w = canvas.width() as f32;

//                     let r = (x * 255. / w).floor() as u8;
//                     let g = (y * 255. / h).floor() as u8;
//                     let b = 255 - ((x + y) * 255. / (h + w)).floor() as u8;
//                     Color::Rgb { r, g, b }
//                 } else {
//                     Color::Named(NamedColor::White)
//                 };
//                 canvas.fill_rect(x, y, color);
//             }
//         }
//         canvas.context.set_fill_style_str(&Color::Named(NamedColor::Black).to_css_color());
//         canvas.context.set_font("120px bold");
//         canvas.context.fill_text(&format!("{frame_counter}"), 10., 120.).unwrap();
//         frame_counter += 1;
//         frame_counter > N_FRAMES
//     };

//     let before = web_sys::window().unwrap().performance().unwrap().now();

//     canvas.play_animation(animation).await;

//     let after = web_sys::window().unwrap().performance().unwrap().now();
//     let delta_secs = (after - before) / 1000.;
//     log!("took {:.2}s", delta_secs);
//     log!("avg fps: {:.2}", N_FRAMES as f64 / delta_secs);
// }

use std::{collections::HashMap, f64};
use wasm_bindgen::prelude::*;

const DEFAULT_CELL_SIZE: f64 = 40.;

pub struct Canvas {
    context: web_sys::CanvasRenderingContext2d,
    cell_size: f64,
    width: usize,
    height: usize,
    canvas_width: usize,
    canvas_height: usize,
    queue: Vec<DrawCall>,
    last_frame: Vec<Vec<Option<Color>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamedColor {
    White,
    Black,
    // TODO: the rest
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    Rgb { r: u8, g: u8, b: u8 },
    Rgba { r: u8, g: u8, b: u8, a: u8 },
    Named(NamedColor),
}
impl Color {
    fn to_css_color(self) -> String {
        match self {
            Color::Rgb { r, g, b } => format!("#{r:0>2X}{g:0>2X}{b:0>2X}"),
            Color::Rgba { r, g, b, a } => format!("#{r:0>2X}{g:0>2X}{b:0>2X}{a:0>2X}"),
            Color::Named(named_color) => format!("{named_color:?}").to_lowercase(),
        }
    }

    fn invert(self) -> Self {
        match self {
            Color::Rgb { r: _, g: _, b: _ } => unimplemented!(),
            Color::Rgba {
                r: _,
                g: _,
                b: _,
                a: _,
            } => unimplemented!(),
            Color::Named(NamedColor::White) => Color::Named(NamedColor::Black),
            Color::Named(NamedColor::Black) => Color::Named(NamedColor::White),
        }
    }
}

/// queued rectangle draw call
#[derive(Clone)]
struct DrawCall {
    x: usize,
    y: usize,
    color: Color,
}

impl Canvas {
    pub fn get_element_by_id(id: &str) -> Option<Self> {
        let document = web_sys::window()?.document()?;
        let canvas = document.get_element_by_id(id)?;
        let canvas: web_sys::HtmlCanvasElement =
            canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok()?;

        let context = canvas
            .get_context("2d")
            .ok()??
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()?;

        let mut res = Self {
            context,
            cell_size: DEFAULT_CELL_SIZE,
            width: 0,
            height: 0,
            canvas_width: canvas.width() as usize,
            canvas_height: canvas.height() as usize,
            queue: vec![],
            last_frame: vec![vec![]],
        };
        res.calculate_size();
        Some(res)
    }

    pub fn with_cell_size(mut self, cell_size: f64) -> Self {
        self.cell_size = cell_size;
        self.calculate_size();
        self
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, color: Color) {
        self.queue.push(DrawCall { x, y, color });
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn calculate_size(&mut self) {
        self.width = (self.canvas_width as f64 / self.cell_size).ceil() as usize;
        self.height = (self.canvas_height as f64 / self.cell_size).ceil() as usize;
        self.last_frame = vec![vec![None; self.height]; self.width]
    }

    /// animation: function that renders a single frame and returns true if it is done
    pub async fn play_animation(
        mut self,
        mut animation: impl FnMut(&mut Canvas) -> bool + 'static,
    ) {
        let step = Closure::new(move || {
            let res = animation(&mut self);
            self.flush();
            res
        });
        start_animation(&step).await;
    }

    fn optimise_queue(&mut self) {
        // 1. remove dupplicate draw calls to the same cell on the same frame
        let mut map = HashMap::new();
        for draw in &self.queue {
            map.insert((draw.x, draw.y), draw.color);
        }
        self.queue.clear();
        for ((x, y), color) in map {
            self.queue.push(DrawCall { x, y, color });
        }

        // 2. remove calls for unchanged cells since last frame
        self.queue
            .retain(|draw| Some(draw.color) != self.last_frame[draw.x][draw.y]);
        // 3. order calls by color to avoid changing the pen color each call
        self.queue.sort_unstable_by_key(|draw| draw.color);
    }

    pub fn flush(&mut self) {
        self.optimise_queue();
        for draw_call in &self.queue {
            let DrawCall { x, y, color } = draw_call;
            self.context
                .set_fill_style_str(&color.invert().to_css_color());
            self.context.fill_rect(
                *x as f64 * self.cell_size - 1.,
                *y as f64 * self.cell_size - 1.,
                self.cell_size + 2.,
                self.cell_size + 2.,
            );
            self.context.set_fill_style_str(&color.to_css_color());
            self.context.fill_rect(
                *x as f64 * self.cell_size,
                *y as f64 * self.cell_size,
                self.cell_size,
                self.cell_size,
            );
            self.last_frame[*x][*y] = Some(*color);
        }
    }
}

#[wasm_bindgen(module = "/lib.js")]
extern "C" {
    async fn start_animation(animation_step: &Closure<dyn FnMut() -> bool>);
}

#[cfg(test)]
mod tests {
    use super::{Color, NamedColor};
    use rstest::rstest;

    #[rstest]
    #[case(Color::Named(NamedColor::Black), "black")]
    #[case(Color::Named(NamedColor::White), "white")]
    #[case(Color::Rgb{r: 255, g: 255, b: 255}, "#FFFFFF")]
    #[case(Color::Rgb{r: 1, g: 2, b: 3}, "#010203")]
    #[case(Color::Rgb{r: 0, g: 0, b: 0}, "#000000")]
    #[case(Color::Rgba{r: 1, g: 2, b: 3, a: 4}, "#01020304")]
    pub fn test_color_to_css_string(#[case] color: Color, #[case] expected_str: &str) {
        assert_eq!(color.to_css_color(), expected_str);
    }
}
