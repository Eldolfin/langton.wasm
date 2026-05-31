use web_sys::Document;

const HTML_ID_CANVAS_PARENT: &str = "langtonrs-canvas-parent";
pub const HTML_ID_CANVAS: &str = "langtonrs-canvas";

pub fn get_canvas_parent() -> Option<web_sys::Element> {
    let document = web_sys::window()?.document()?;
    let body = document.body().unwrap();
    let parent_el = match document.get_element_by_id(HTML_ID_CANVAS_PARENT) {
        Some(parent) => parent,
        None => {
            let new_parent = document.create_element("div").unwrap();
            new_parent.set_id(HTML_ID_CANVAS_PARENT);
            body.prepend_with_node_1(&new_parent).unwrap();
            new_parent
        }
    };
    Some(parent_el)
}

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

pub fn document() -> Document {
    window()
        .document()
        .expect("should have a document on window")
}

pub fn url() -> url::Url {
    document().url().unwrap().parse().unwrap()
}

#[macro_export]
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log!("{}", &format_args!($($t)*).to_string()))
}
