use common::get_canvas_parent;
use debug_ui::Param;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console::warn_1, wasm_bindgen::prelude::*, window};

#[wasm_bindgen(inline_js = "
export function batch_fill_rects(ctx, data) {
    const len = data.length;
    let i = 0;
    while (i < len) {
        const x = data[i];
        const y = data[i + 1];
        const w = data[i + 2];
        const h = data[i + 3];
        const r = data[i + 4];
        const g = data[i + 5];
        const b = data[i + 6];
        const a = data[i + 7];
        ctx.fillStyle = 'rgba(' + r + ',' + g + ',' + b + ',' + (a / 255) + ')';
        ctx.fillRect(x, y, w, h);
        i += 8;
    }
}
")]
extern "C" {
    fn batch_fill_rects(ctx: &web_sys::CanvasRenderingContext2d, data: &js_sys::Uint16Array);
}

pub struct Canvas {
    element: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
    /// render calls queue
    queue: Vec<DrawCall>,
    /// flat 1D dedup buffer indexed by `x * height + y`, reused each frame
    dedup_vec: Vec<Option<Color>>,
    /// indices into dedup_vec written this frame; cleared after each optimise_queue
    dedup_dirty: Vec<usize>,
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
    /// Persistent buffer for flush, reused across frames to avoid per-frame allocation
    flush_buf: Vec<u16>,
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
    pub fn to_css_color(self) -> String {
        match self {
            Color::Rgb { r, g, b } => format!("#{r:0>2X}{g:0>2X}{b:0>2X}"),
            Color::Rgba { r, g, b, a } => format!("rgba({r}, {g}, {b}, {})", a as f32 / 255.0),
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

fn color_components(color: Color) -> (u8, u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b, 255),
        Color::Rgba { r, g, b, a } => (r, g, b, a),
        Color::Named(NamedColor::White) => (255, 255, 255, 255),
        Color::Named(NamedColor::Black) => (0, 0, 0, 255),
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
            dedup_vec: vec![],
            dedup_dirty: vec![],
            last_frame: vec![],
            cell_border_size,
            width: 0,
            height: 0,
            screen_height: 0,
            last_cell_size: 0,
            flush_buf: vec![],
        }
    }

    pub fn new_with_element(
        element: web_sys::HtmlCanvasElement,
        cell_border_size: Rc<RefCell<Param<usize>>>,
        cell_size: Rc<RefCell<Param<usize>>>,
    ) -> Self {
        let context = Self::get_context(&element).expect("Failed to get context 2d");
        let base_screen_height = element.height() as usize;

        Self {
            canvas_width: element.width() as usize,
            canvas_height: element.height() as usize,
            element,
            context,
            cell_size,
            base_screen_height,
            queue: vec![],
            dedup_vec: vec![],
            dedup_dirty: vec![],
            last_frame: vec![],
            cell_border_size,
            width: 0,
            height: 0,
            screen_height: 0,
            last_cell_size: 0,
            flush_buf: vec![],
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
        let cell_size = self.cell_size.borrow().get();
        self.last_cell_size = cell_size;
        self.width = (self.canvas_width as f64 / cell_size as f64).ceil() as usize;
        self.height = (self.canvas_height as f64 / cell_size as f64).ceil() as usize;
        self.screen_height = (self.base_screen_height as f64 / cell_size as f64).ceil() as usize;
        self.last_frame = vec![vec![None; self.height]; self.width];
        self.dedup_vec = vec![None; self.width * self.height];
        // Discard any queued draw calls that used the old cell dimensions.
        // Keeping stale coordinates could cause out-of-bounds access in flush().
        self.queue.clear();
    }

    fn calculate_size_if_needed(&mut self) {
        if self.cell_size.borrow().get() != self.last_cell_size {
            self.calculate_size();
            assert!(self.width > 0);
            assert!(self.height > 0);
        }
    }

    /// animation: function that renders a single frame and returns true if it is done
    pub async fn play_animation(&mut self, mut animation: impl FnMut(&mut Canvas) -> bool) {
        loop {
            // Wait for next animation frame
            let promise = web_sys::js_sys::Promise::new(&mut |resolve, _| {
                window()
                    .unwrap()
                    .request_animation_frame(&resolve)
                    .expect("should register `requestAnimationFrame` OK");
            });
            JsFuture::from(promise).await.unwrap();

            // Do one frame
            self.calculate_size_if_needed();
            let done = animation(self);
            self.flush();
            if done {
                break;
            }
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.context.set_fill_style_str(&color.to_css_color());
        self.context.fill_rect(
            0.0,
            0.0,
            self.canvas_width as f64,
            self.canvas_height as f64,
        );
        // Reset last_frame so subsequent draws won't be skipped by dedup
        for col in &mut self.last_frame {
            for cell in col.iter_mut() {
                *cell = None;
            }
        }
        self.queue.clear();
    }

    pub fn fill_canvas(&mut self, retention_factor: u8, bg_color: Option<Color>) {
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
        let fade_color = Color::Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: retention_factor,
        };

        // 4. Set fill style and draw the rectangle.
        self.context.set_fill_style_str(&fade_color.to_css_color());
        self.context.fill_rect(
            0.0,
            0.0,
            self.canvas_width as f64,
            self.canvas_height as f64,
        );

        // 5. Optionally draw the background behind.
        if let Some(bg_color) = bg_color {
            let _ = self
                .context
                .set_global_composite_operation("destination-over");
            self.context.set_fill_style_str(&bg_color.to_css_color());
            self.context.fill_rect(
                0.0,
                0.0,
                self.canvas_width as f64,
                self.canvas_height as f64,
            );
        }

        // 6. Restore the original globalCompositeOperation.
        let _ = self.context.set_global_composite_operation(&original_gco);
    }

    fn optimise_queue(&mut self) {
        // 1. remove dupplicate draw calls to the same cell on the same frame
        for draw in &self.queue {
            if draw.x >= self.width || draw.y >= self.height {
                continue;
            }
            let idx = draw.x * self.height + draw.y;
            if self.dedup_vec[idx].is_none() {
                self.dedup_dirty.push(idx);
            }
            self.dedup_vec[idx] = Some(draw.color);
        }
        self.queue.clear();
        for &idx in &self.dedup_dirty {
            let color = self.dedup_vec[idx].take().unwrap();
            self.queue.push(DrawCall {
                x: idx / self.height,
                y: idx % self.height,
                color,
            });
        }
        self.dedup_dirty.clear();

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
        if self.queue.is_empty() {
            return;
        }

        let cell_size = self.cell_size.borrow().get();
        let border_size = self.cell_border_size.borrow().get();
        let border_size = if cell_size <= 2 * border_size {
            0
        } else {
            border_size
        };
        if border_size == 0 {
            let buf = &mut self.flush_buf;
            buf.clear();
            buf.reserve(self.queue.len() * 8);

            for draw_call in &self.queue {
                let DrawCall { x, y, color } = draw_call;
                let cs = cell_size as u8;
                let ix = (*x * cell_size) as u16;
                let iy = (*y * cell_size) as u16;
                let (r, g, b, a) = color_components(*color);
                buf.extend_from_slice(&[
                    ix, iy, cs as u16, cs as u16, r as u16, g as u16, b as u16, a as u16,
                ]);
                self.last_frame[*x][*y] = Some(*color);
            }

            let js_array = js_sys::Uint16Array::from(buf.as_slice());
            batch_fill_rects(&self.context, &js_array);
        } else {
            let buf = &mut self.flush_buf;
            buf.clear();
            buf.reserve(self.queue.len() * 2 * 8);

            for draw_call in &self.queue {
                let DrawCall { x, y, color } = draw_call;
                let cs = cell_size as u8;
                let ix = (*x * cell_size) as u16;
                let iy = (*y * cell_size) as u16;

                let inv = color.invert();
                let (r, g, b, a) = color_components(inv);
                buf.extend_from_slice(&[
                    ix, iy, cs as u16, cs as u16, r as u16, g as u16, b as u16, a as u16,
                ]);

                let (r, g, b, a) = color_components(*color);
                let bs = border_size as u8;
                let inner_size = (cs - 2 * bs) as u16;
                buf.extend_from_slice(&[
                    ix + border_size as u16,
                    iy + border_size as u16,
                    inner_size,
                    inner_size,
                    r as u16,
                    g as u16,
                    b as u16,
                    a as u16,
                ]);
                self.last_frame[*x][*y] = Some(*color);
            }

            let js_array = js_sys::Uint16Array::from(buf.as_slice());
            batch_fill_rects(&self.context, &js_array);
        }
    }

    fn create_canvas() -> Option<web_sys::HtmlCanvasElement> {
        let document = web_sys::window()?.document()?;
        let body = document.body().unwrap();
        let parent_el = get_canvas_parent()?;
        // FIXME: style is appended multiple times
        let style = document.create_element("style").unwrap();
        style.set_text_content(Some(include_str!("./style.css")));
        document.head().unwrap().append_child(&style).unwrap();
        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()?;
        canvas.set_id(common::HTML_ID_CANVAS);
        parent_el.prepend_with_node_1(&canvas).unwrap();
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
    #[case(Color::Rgba{r: 1, g: 2, b: 3, a: 255}, "rgba(1, 2, 3, 1)")]
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
