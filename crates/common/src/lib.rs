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
