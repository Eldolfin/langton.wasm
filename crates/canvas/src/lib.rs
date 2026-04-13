use debug_ui::{Param, log};
use std::{cell::RefCell, collections::HashMap, f64, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console::warn_1, window};

pub struct Canvas {
    element: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
    /// render calls queue
    queue: Vec<DrawCall>,
    last_frame: Vec<Vec<Option<Color>>>,
    /// in pixels
    cell_size: Rc<RefCell<debug_ui::Param<usize>>>,
    /// in pixels
    cell_border_size: Rc<RefCell<debug_ui::Param<usize>>>,
    /// in cells
    width: usize,
    /// in cells
    height: usize,
    /// in cells
    screen_height: usize,
    /// in pixels
    base_screen_height: usize,
    /// in pixels
    canvas_width: usize,
    /// in pixels
    canvas_height: usize,
    last_cell_size: usize,
}

impl Drop for Canvas {
    fn drop(&mut self) {
        self.element.remove();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamedColor {
    White,
    Black,
    // TODO: the rest
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
            Color::Rgb { r, g, b } => Color::Rgb {
                r: 255 - r,
                g: 255 - g,
                b: 255 - b,
            },
            Color::Rgba { r, g, b, a } => Color::Rgba {
                r: 255 - r,
                g: 255 - g,
                b: 255 - b,
                a, // Preserve alpha
            },
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
    pub fn new(
        cell_border_size: Rc<RefCell<Param<usize>>>,
        cell_size: Rc<RefCell<Param<usize>>>,
    ) -> Self {
        let Some(canvas) = Self::create_canvas() else {
            panic!("Failed to get canvas!")
        };
        let Some(context) = Self::get_context(&canvas) else {
            panic!("Failed to get context 2d out of canvas!")
        };

        let base_screen_height =
            window().unwrap().inner_height().unwrap().as_f64().unwrap() as usize;
        let base_screen_height = std::cmp::min(canvas.height() as usize, base_screen_height);

        Self {
            element: canvas.clone(),
            context,
            cell_size,
            canvas_width: canvas.width() as usize,
            canvas_height: canvas.height() as usize,
            base_screen_height,
            queue: vec![],
            last_frame: vec![],
            cell_border_size,
            width: 0,
            height: 0,
            screen_height: 0,
            last_cell_size: 0,
        }
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

    pub fn screen_height(&self) -> usize {
        self.screen_height
    }

    fn calculate_size(&mut self) {
        let cell_size = self.cell_size.borrow_mut().get();
        self.last_cell_size = cell_size;
        self.width = (self.canvas_width as f64 / cell_size as f64).ceil() as usize;
        self.height = (self.canvas_height as f64 / cell_size as f64).ceil() as usize;
        self.screen_height = (self.base_screen_height as f64 / cell_size as f64).ceil() as usize;
        self.last_frame = vec![vec![None; self.height]; self.width];
        // Discard any queued draw calls that used the old cell dimensions.
        // Keeping stale coordinates could cause out-of-bounds access in flush().
        self.queue.clear();
    }

    fn calculate_size_if_needed(&mut self) {
        if self.cell_size.borrow_mut().get() != self.last_cell_size {
            self.calculate_size();
            assert!(self.width > 0);
            assert!(self.height > 0);
        }
    }

    /// animation: function that renders a single frame and returns true if it is done
    pub async fn play_animation(
        selff: Rc<RefCell<Self>>,
        mut animation: impl FnMut(&mut Canvas) -> bool + 'static,
    ) {
        let step = move || {
            let mut selff = selff.borrow_mut();
            selff.calculate_size_if_needed();
            let res = animation(&mut selff);
            selff.flush();
            res
        };
        let step = Rc::new(RefCell::new(step));
        start_animation(step).await;
    }

    pub fn fill_canvas(&mut self, retention_factor: u8) {
        // 1. Get and store the current globalCompositeOperation.
        let original_gco = self
            .context
            .global_composite_operation()
            .unwrap_or_else(|_err| "source-over".to_string());

        // 2. Set globalCompositeOperation to "destination-in".
        let _ = self
            .context
            .set_global_composite_operation("destination-in");

        // 3. Construct the color for fading. This will make existing content fade to transparent black.
        let color = Color::Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: retention_factor,
        };

        // 4. Set fill style and draw the rectangle.
        self.context.set_fill_style_str(&color.to_css_color());
        self.context.fill_rect(
            0.0,
            0.0,
            self.canvas_width as f64,
            self.canvas_height as f64,
        );

        // 5. Restore the original globalCompositeOperation.
        let _ = self.context.set_global_composite_operation(&original_gco);
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

        // 2. remove calls for unchanged cells since last frame, and drop any
        // out-of-bounds calls that may arise when cell_size changes mid-frame.
        self.queue.retain(|draw| {
            draw.x < self.last_frame.len()
                && draw.y < self.last_frame.get(draw.x).map_or(0, |col| col.len())
                && Some(draw.color) != self.last_frame[draw.x][draw.y]
        });
        // 3. order calls by color to avoid changing the pen color each call
        self.queue.sort_unstable_by_key(|draw| draw.color);
    }

    pub fn flush(&mut self) {
        self.optimise_queue();
        let cell_size = self.cell_size.borrow_mut().get();
        let raw_border_size = self.cell_border_size.borrow_mut().get();
        let border_size = if cell_size <= 2 * raw_border_size {
            0
        } else {
            raw_border_size
        };
        for draw_call in &self.queue {
            let DrawCall { x, y, color } = draw_call;
            // avoid calling the "expensive" fill_rect if there is no border
            if raw_border_size != 0 {
                self.context
                    .set_fill_style_str(&color.invert().to_css_color());
                self.context.fill_rect(
                    (*x * cell_size) as f64,
                    (*y * cell_size) as f64,
                    (cell_size) as f64,
                    (cell_size) as f64,
                );
            }
            self.context.set_fill_style_str(&color.to_css_color());
            // center
            self.context.fill_rect(
                (*x * cell_size + border_size) as f64,
                (*y * cell_size + border_size) as f64,
                (cell_size - 2 * border_size) as f64,
                (cell_size - 2 * border_size) as f64,
            );
            self.last_frame[*x][*y] = Some(*color);
        }
    }
    fn create_canvas() -> Option<web_sys::HtmlCanvasElement> {
        let document = web_sys::window()?.document()?;
        let body = document.body().unwrap();
        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()?;
        body.prepend_with_node_1(&canvas).unwrap();
        let style = document.create_element("style").unwrap();
        style.set_text_content(Some(include_str!("./style.css")));
        document.head().unwrap().append_child(&style).unwrap();
        let scroll_height = body.scroll_height() as u32;
        let canvas_height = if scroll_height > 0 {
            scroll_height
        } else {
            warn_1(
            &"[LANGTON][CANVAS] body.scroll_height is 0, make sure to fully initialize the page before calling start_langton_ant otherwise the canvas might get cut off at the bottom".into()
        );
            window().unwrap().inner_height().unwrap().as_f64().unwrap() as u32
        };
        canvas.set_width(window().unwrap().inner_width().unwrap().as_f64().unwrap() as u32);
        canvas.set_height(canvas_height);
        Some(canvas)
    }

    fn get_context(
        canvas: &web_sys::HtmlCanvasElement,
    ) -> Option<web_sys::CanvasRenderingContext2d> {
        canvas
            .get_context("2d")
            .ok()??
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

async fn start_animation(animation_step: Rc<RefCell<impl FnMut() -> bool + 'static>>) {
    let promise = web_sys::js_sys::Promise::new(&mut |resolve, _reject| {
        let update = Rc::new(RefCell::new(None));
        let f = update.clone();
        let value = animation_step.clone();
        *f.borrow_mut() = Some(Closure::new(move || {
            let res = value.borrow_mut()();
            log!("{res:?}");
            if !res {
                request_animation_frame(update.borrow_mut().as_ref().unwrap());
            } else {
                // free closure
                let _ = update.borrow_mut().take();
                resolve.call0(&JsValue::NULL).unwrap();
            }
        }));
        request_animation_frame(f.borrow_mut().as_ref().unwrap());
    });
    JsFuture::from(promise).await.unwrap();
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

    #[rstest]
    #[case(Color::Rgb { r: 0, g: 0, b: 0 }, Color::Rgb { r: 255, g: 255, b: 255 })]
    #[case(Color::Rgb { r: 255, g: 255, b: 255 }, Color::Rgb { r: 0, g: 0, b: 0 })]
    #[case(Color::Rgb { r: 10, g: 20, b: 30 }, Color::Rgb { r: 245, g: 235, b: 225 })]
    #[case(Color::Rgba { r: 0, g: 0, b: 0, a: 100 }, Color::Rgba { r: 255, g: 255, b: 255, a: 100 })]
    #[case(Color::Rgba { r: 255, g: 255, b: 255, a: 50 }, Color::Rgba { r: 0, g: 0, b: 0, a: 50 })]
    #[case(Color::Rgba { r: 10, g: 20, b: 30, a: 0 }, Color::Rgba { r: 245, g: 235, b: 225, a: 0 })]
    #[case(Color::Named(NamedColor::White), Color::Named(NamedColor::Black))]
    #[case(Color::Named(NamedColor::Black), Color::Named(NamedColor::White))]
    fn test_color_invert(#[case] original: Color, #[case] expected_inverted: Color) {
        assert_eq!(original.invert(), expected_inverted);
    }
}
