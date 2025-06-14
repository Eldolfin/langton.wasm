use gloo::events::EventListener;
use num_traits::{FromPrimitive, Num, ToPrimitive};
use std::{collections::HashMap, ops::Range, str::FromStr, sync::mpsc};
pub use web_sys;
use web_sys::{Document, Element, HtmlInputElement, wasm_bindgen::JsCast as _};
pub use debug_ui_derive::Config;

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        debug_ui::web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

#[macro_export]
macro_rules! warn {
    ( $( $t:tt )* ) => {
        web_sys::console::warn_1(&format!( $( $t )* ).into())
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

/// options for the param function
#[derive(Clone)]
pub struct ParamParam<T, S> {
    pub name: S,
    pub default_value: T,
    pub range: Range<T>,
    pub scale: Scale,
    pub step_size: f64,
}

impl<T: Num> Default for ParamParam<T, &str> {
    fn default() -> Self {
        let is_float = T::one() / (T::one() + T::one()) != T::zero();
        let step_size = if is_float { 0.0 } else { 1.0 };
        Self {
            name: "UNDEFINED 🤡",
            default_value: T::zero(),
            range: T::zero()..T::one(),
            scale: Scale::default(),
            step_size,
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub enum Scale {
    #[default]
    Linear,
    Logarithmic,
}

impl<T: Copy> Param<T> {
    fn new(value: T) -> (mpsc::Sender<T>, Self) {
        let (send, recv) = mpsc::channel();
        (send, Self { recv, value })
    }

    pub fn get(&mut self) -> T {
        while let Ok(val) = self.recv.try_recv() {
            self.value = val;
        }
        self.value
    }
}

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn document() -> Document {
    window()
        .document()
        .expect("should have a document on window")
}

#[cfg(any(feature = "auto-detect-path-params", feature = "save-params-in-url"))]
fn url() -> url::Url {
    document().url().unwrap().parse().unwrap()
}

#[cfg(feature = "save-params-in-url")]
fn add_url_param<T: Copy + ToString + FromStr + ToPrimitive + FromPrimitive + 'static>(
    key: &str,
    value: T,
) {
    use web_sys::wasm_bindgen::JsValue;

    let mut new_url = url();
    let mut params: HashMap<String, String> = new_url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    // remove old parameter
    params.retain(|k, _| k != key);
    params.insert(key.into(), value.to_string());
    new_url.query_pairs_mut().clear();
    let mut params: Vec<_> = params.into_iter().collect();
    params.sort();
    new_url.query_pairs_mut().extend_pairs(params);
    window()
        .history()
        .unwrap()
        .push_state_with_url(&JsValue::NULL, "", Some(new_url.as_str()))
        .unwrap();
}

impl DebugUI {
    pub fn new(title: &str) -> Self {
        let document = document();
        #[cfg(feature = "auto-detect-path-params")]
        if !url().query_pairs().any(|param| param.0 == "debug") {
            return Self::Disabled;
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

    pub fn param<
        T: Copy + ToString + FromStr + ToPrimitive + FromPrimitive + 'static,
        S: AsRef<str> + Clone,
    >(
        &mut self,
        p: ParamParam<T, S>,
    ) -> Param<T> {
        let key = p.name.as_ref().replace(" ", "_");
        #[cfg(not(feature = "save-params-in-url"))]
        let default_value = p.default_value;
        #[cfg(feature = "save-params-in-url")]
        let default_value = url()
            .query_pairs()
            .find(|(k, _)| k.as_ref() == key)
            .map(|(_, v)| v.parse())
            .into_iter()
            .flatten()
            .next()
            .unwrap_or(p.default_value);

        let (send, param_value) = Param::new(default_value);
        match self {
            DebugUI::Enabled {
                root,
                document: doc,
                next_uid,
            } => {
                let container = doc.create_element("div").unwrap();
                let label = doc.create_element("label").unwrap();
                let slider = doc
                    .create_element("input")
                    .unwrap()
                    .dyn_into::<HtmlInputElement>()
                    .unwrap();
                let value_input = doc
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
                label.set_text_content(Some(p.name.as_ref()));
                label.set_attribute("for", &slider_id).unwrap();
                value_input.set_value_as_number(default_value.to_f64().unwrap());

                {
                    let (min, max, step) = match p.scale {
                        Scale::Linear => (
                            p.range.start.to_f64().unwrap(),
                            p.range.end.to_f64().unwrap(),
                            if p.step_size == 0.0 {
                                "any".to_string()
                            } else {
                                p.step_size.to_string()
                            },
                        ),
                        Scale::Logarithmic => (0.0, 1.0, "any".to_string()),
                    };
                    slider.set_attribute("min", &min.to_string()).unwrap();
                    slider.set_attribute("max", &max.to_string()).unwrap();
                    slider.set_attribute("step", &step).unwrap();
                }
                slider.set_value_as_number(p.scale.unscale(default_value, &p.range));

                container.set_class_name("DebugUI-param-container");
                label.set_class_name("DebugUI-param-label");
                slider.set_class_name("DebugUI-param-slider");
                value_input.set_class_name("DebugUI-param-value");

                container.append_child(&label).unwrap();
                container.append_child(&slider).unwrap();
                container.append_child(&value_input).unwrap();
                root.append_child(&container).unwrap();

                {
                    let document = doc.clone();
                    let name = p.name.as_ref().to_owned();
                    let value_id = value_id.clone();
                    let slider_id = slider_id.clone();
                    let send = send.clone();
                    let p = p.clone();
                    let key = key.clone();
                    EventListener::new(&slider, "input", move |_event| {
                        let value = document
                            .get_element_by_id(&slider_id)
                            .unwrap()
                            .dyn_into::<HtmlInputElement>()
                            .unwrap()
                            .value_as_number();
                        let scaled = p.scale.scale(value, &p.range);
                        let value_input = document
                            .get_element_by_id(&value_id)
                            .unwrap()
                            .dyn_into::<HtmlInputElement>()
                            .unwrap();

                        value_input.set_value_as_number(scaled);

                        let value = T::from_f64(scaled).unwrap_or_else(|| {
                            panic!("Failed to cast slider value for parameter {name}")
                        });

                        #[cfg(feature = "save-params-in-url")]
                        add_url_param(&key, value);

                        send.send(value).unwrap();
                    })
                    .forget();
                }
                {
                    let doc = doc.clone();
                    let name = p.name.as_ref().to_owned();
                    let value_id = value_id.clone();
                    let slider_id = slider_id.clone();
                    let send = send.clone();
                    let p = p.clone();
                    let key = key.clone();
                    EventListener::new(&value_input, "change", move |_event| {
                        let value = doc
                            .get_element_by_id(&value_id)
                            .unwrap()
                            .dyn_into::<HtmlInputElement>()
                            .unwrap()
                            .value_as_number();
                        let unscaled = p.scale.unscale(value, &p.range);
                        let slider_input = doc
                            .get_element_by_id(&slider_id)
                            .unwrap()
                            .dyn_into::<HtmlInputElement>()
                            .unwrap();

                        // TODO: add range check here?
                        slider_input.set_value_as_number(unscaled);

                        let value = T::from_f64(value).unwrap_or_else(|| {
                            panic!("Failed to cast slider value for parameter {name}")
                        });

                        #[cfg(feature = "save-params-in-url")]
                        add_url_param(&key, value);

                        send.send(value).unwrap();
                    })
                    .forget();
                }
            }
            DebugUI::Disabled => (),
        }
        param_value
    }
}

impl Scale {
    // these doc strings are only true for Logarithmic scale smh..

    /// - input: a float in the range 0..1
    /// - min: minimum output value
    /// - max: maximum output value
    fn scale<T: ToPrimitive>(self, input: f64, range: &Range<T>) -> f64 {
        match self {
            Scale::Linear => input,
            Scale::Logarithmic => {
                (input * (range.end.to_f64().unwrap() - range.start.to_f64().unwrap() + 1.).ln())
                    .exp()
                    + range.start.to_f64().unwrap()
                    - 1.
            }
        }
    }

    /// - input: a float in the range min..max
    /// - min: minimum output value
    /// - max: maximum output value
    ///
    /// Result:
    /// a float in the range 0..1
    fn unscale<T1: ToPrimitive, T2: ToPrimitive>(self, input: T2, range: &Range<T1>) -> f64 {
        match self {
            Scale::Linear => input.to_f64().unwrap(),
            Scale::Logarithmic => {
                (input.to_f64().unwrap() - range.start.to_f64().unwrap() + 1.).ln()
                    / (range.end.to_f64().unwrap() - range.start.to_f64().unwrap() + 1.).ln()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scale;
    use rstest::rstest;

    #[rstest]
    #[case(Scale::Linear, 0.1, 0., 1000., 0.1)]
    #[case(Scale::Linear, 1000., 0., 0., 1000.)] // validation is not this function's job
    #[case(Scale::Logarithmic, 0., 0., 1000., 0.)]
    #[case(Scale::Logarithmic, 1., 0., 1000., 1000.)]
    #[case(Scale::Logarithmic, 0., 526., 527., 526.)]
    #[case(Scale::Logarithmic, 1., 526., 527., 527.)]
    #[case(Scale::Logarithmic, 0.5, 0., 1000., 30.638584039112747)]
    pub fn scale_unscale_test(
        #[case] scale: Scale,
        #[case] input: f64,
        #[case] min: f64,
        #[case] max: f64,
        #[case] output: f64,
    ) {
        const EPSILON: f64 = 1e-7;
        let scaled = scale.scale(input, &(min..max));
        let unscaled = scale.unscale(output, &(min..max));
        assert!(
            (scaled - output).abs() < EPSILON,
            "{scale:?}.scale({input}, {min}, {max}) = {scaled} wanted {output}"
        );
        assert!(
            (unscaled - input).abs() < EPSILON,
            "{scale:?}.unscale({output}, {min}, {max}) = {unscaled} wanted {input}"
        );
    }
}
