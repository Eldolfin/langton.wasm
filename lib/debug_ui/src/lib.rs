use gloo::events::EventListener;
use num_traits::{FromPrimitive, ToPrimitive};
use std::{ops::Range, sync::mpsc};
use web_sys::{wasm_bindgen::JsCast as _, Document, Element, HtmlInputElement};

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub enum DebugUI {
    Enabled {
        root: Element,
        document: Document,
        next_uid: u32,
    },
    Disabled,
}

pub struct Param<T> {
    value: T,
    recv: mpsc::Receiver<T>,
}

impl<T: Copy> Param<T> {
    fn new(value: T) -> (mpsc::SyncSender<T>, Self) {
        let (send, recv) = mpsc::sync_channel(32);
        (send, Self { recv, value })
    }

    pub fn get(&mut self) -> T {
        while let Ok(val) = self.recv.try_recv() {
            self.value = val;
        }
        self.value
    }
}

impl DebugUI {
    pub fn new(title: &str) -> Self {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        #[cfg(feature = "auto-detect-path-params")]
        {
            let url: url::Url = document.url().unwrap().parse().unwrap();
            let debug_enabled = url.query_pairs().any(|param| param.0 == "debug");
            if !debug_enabled {
                return Self::Disabled;
            }
        }
        let body = document.body().expect("document should have a body");

        let root = document.create_element("div").unwrap();
        let title_btn = document.create_element("h2").unwrap();

        title_btn.set_text_content(Some(title));
        root.set_class_name("DebugUI-root-box");
        title_btn.set_class_name("DebugUI-title-btn");

        root.append_child(&title_btn).unwrap();
        body.append_child(&root).unwrap();

        let style = document.create_element("style").unwrap();
        style.set_text_content(Some(include_str!("./style.css")));
        document.head().unwrap().append_child(&style).unwrap();

        Self::Enabled {
            root,
            document,
            next_uid: 0,
        }
    }

    pub fn param<T: ToPrimitive + FromPrimitive + Copy + Default + 'static + ToString>(
        &mut self,
        name: &str,
        default_value: T,
        range: Range<T>,
    ) -> Param<T> {
        let (send, param_value) = Param::new(default_value);
        match self {
            DebugUI::Enabled {
                root,
                document,
                next_uid,
            } => {
                let container = document.create_element("div").unwrap();
                let label = document.create_element("label").unwrap();
                let slider = document
                    .create_element("input")
                    .unwrap()
                    .dyn_into::<HtmlInputElement>()
                    .unwrap();
                let value_input = document
                    .create_element("input")
                    .unwrap()
                    .dyn_into::<HtmlInputElement>()
                    .unwrap();

                let uid = *next_uid;
                *next_uid += 1;
                let slider_id = format!("debugui-slider-{uid}");
                let value_id = format!("debugui-value-{uid}");

                slider.set_id(&slider_id);
                value_input.set_id(&value_id);

                slider.set_attribute("type", "range").unwrap();
                value_input.set_attribute("type", "number").unwrap();
                label.set_text_content(Some(name));
                label.set_attribute("for", &slider_id).unwrap();
                value_input.set_value_as_number(default_value.to_f64().unwrap());
                slider
                    .set_attribute("min", &range.start.to_string())
                    .unwrap();
                slider.set_attribute("max", &range.end.to_string()).unwrap();

                container.set_class_name("DebugUI-param-container");
                label.set_class_name("DebugUI-param-label");
                slider.set_class_name("DebugUI-param-slider");
                value_input.set_class_name("DebugUI-param-value");

                container.append_child(&label).unwrap();
                container.append_child(&slider).unwrap();
                container.append_child(&value_input).unwrap();
                root.append_child(&container).unwrap();

                let document = document.clone();
                let name = name.to_owned();
                let on_change = EventListener::new(&slider, "input", move |_event| {
                    let value = document
                        .get_element_by_id(&slider_id)
                        .unwrap()
                        .dyn_into::<HtmlInputElement>()
                        .unwrap()
                        .value_as_number();
                    let value_input = document
                        .get_element_by_id(&value_id)
                        .unwrap()
                        .dyn_into::<HtmlInputElement>()
                        .unwrap();

                    value_input.set_value_as_number(value);

                    let value = T::from_f64(value).unwrap_or_else(|| {
                        panic!("Failed to cast slider value for parameter {name}")
                    });
                    send.send(value).unwrap();
                });

                // When a Closure is dropped it will invalidate the associated JS closure.
                // Here we want JS callback to be alive for the entire duration of the program.
                // So we used `forget` leak this instance of Closure.
                // It should be used sparingly to ensure the memory leak doesn't affect the program too much.
                on_change.forget();
            }
            DebugUI::Disabled => (),
        }
        param_value
    }
}
