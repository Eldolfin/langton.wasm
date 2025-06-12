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

/// options for the param function
#[derive(Clone)]
pub struct ParamParam<T, S>
where
    T: Clone,
    S: Clone,
{
    pub name: S,
    pub default_value: T,
    pub range: Range<T>,
    pub scale: Scale,
}

#[derive(Clone, Copy, Default, Debug)]
pub enum Scale {
    #[default]
    Linear,
    Logarithmic,
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

    pub fn param<
        T: ToPrimitive + FromPrimitive + Copy + Default + 'static + ToString + Clone + std::fmt::Debug,
        S: AsRef<str> + Clone,
    >(
        &mut self,
        p: ParamParam<T, S>,
    ) -> Param<T> {
        let (send, param_value) = Param::new(p.default_value);
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
                label.set_text_content(Some(p.name.as_ref()));
                label.set_attribute("for", &slider_id).unwrap();
                value_input.set_value_as_number(p.default_value.to_f64().unwrap());
                slider.set_value_as_number(p.scale.unscale(p.default_value, &p.range));

                {
                    let (min, max, step) = match p.scale {
                        Scale::Linear => (
                            p.range.start.to_f64().unwrap(),
                            p.range.end.to_f64().unwrap(),
                            "1", // TODO: add precision parameter
                        ),
                        Scale::Logarithmic => (0.0, 1.0, "any"),
                    };
                    slider.set_attribute("min", &min.to_string()).unwrap();
                    slider.set_attribute("max", &max.to_string()).unwrap();
                    slider.set_attribute("step", step).unwrap();
                }

                container.set_class_name("DebugUI-param-container");
                label.set_class_name("DebugUI-param-label");
                slider.set_class_name("DebugUI-param-slider");
                value_input.set_class_name("DebugUI-param-value");

                container.append_child(&label).unwrap();
                container.append_child(&slider).unwrap();
                container.append_child(&value_input).unwrap();
                root.append_child(&container).unwrap();

                {
                    let document = document.clone();
                    let name = p.name.as_ref().to_owned();
                    let value_id = value_id.clone();
                    let slider_id = slider_id.clone();
                    let send = send.clone();
                    let p = p.clone();
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
                        send.send(value).unwrap();
                    })
                    .forget();
                }
                {
                    let document = document.clone();
                    let name = p.name.as_ref().to_owned();
                    let value_id = value_id.clone();
                    let slider_id = slider_id.clone();
                    let send = send.clone();
                    let p = p.clone();
                    EventListener::new(&value_input, "change", move |_event| {
                        let value = document
                            .get_element_by_id(&value_id)
                            .unwrap()
                            .dyn_into::<HtmlInputElement>()
                            .unwrap()
                            .value_as_number();
                        let unscaled = p.scale.unscale(value, &p.range);
                        let slider_input = document
                            .get_element_by_id(&slider_id)
                            .unwrap()
                            .dyn_into::<HtmlInputElement>()
                            .unwrap();

                        // TODO: add range check here?
                        slider_input.set_value_as_number(unscaled);

                        let value = T::from_f64(value).unwrap_or_else(|| {
                            panic!("Failed to cast slider value for parameter {name}")
                        });
                        send.send(value).unwrap();
                    })
                }
                .forget();
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
