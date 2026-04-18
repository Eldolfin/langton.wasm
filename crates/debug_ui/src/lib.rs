use gloo::events::EventListener;
use num_traits::{FromPrimitive, Num, ToPrimitive};
use std::{
    cell::RefCell, collections::HashMap, ops::RangeInclusive, rc::Rc, str::FromStr, sync::mpsc,
};
pub use web_sys;
use web_sys::{Document, Element, HtmlInputElement, KeyboardEvent, wasm_bindgen::JsCast as _};

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

pub enum DebugUIState {
    Enabled {
        root: Element,
        next_uid: u32,
        needs_restart: bool,
    },
    Disabled {
        root: Element,
        next_uid: u32,
    },
}

pub struct DebugUI {
    state: Rc<RefCell<DebugUIState>>,
    _shortcut_listener: EventListener,
    document: Document,
    needs_clear_shared: Rc<RefCell<bool>>,
}

pub struct Param<T> {
    value: T,
    recv: mpsc::Receiver<T>,
}

/// options for the param function
#[derive(Clone)]
pub struct ParamParam<T, S> {
    /// Display name in the panel
    pub name: S,
    /// Starting value, used when values are reset
    pub default_value: T,
    /// Allowed range of the values
    pub range: RangeInclusive<T>,
    /// Optional Logarithmic scale for more freedom of range/precision
    pub scale: Scale,
    /// Allowed precision for sliders
    pub step_size: f64,
    /// When changed, the animation should be restarted for it to take effect
    pub needs_restart: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub enum Scale {
    #[default]
    /// Steps are all of equal value
    Linear,
    /// Steps are much smaller near 0
    Logarithmic,
}

impl<T: Num> Default for ParamParam<T, &str> {
    fn default() -> Self {
        let is_float = T::one() / (T::one() + T::one()) != T::zero();
        let step_size = if is_float { 0.0 } else { 1.0 };
        Self {
            name: "UNDEFINED 🤡",
            default_value: T::zero(),
            range: T::zero()..=T::one(),
            scale: Scale::default(),
            step_size,
            needs_restart: false,
        }
    }
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

pub struct StepCounter {
    element: Option<Element>,
    count: u64,
}

impl StepCounter {
    pub fn add_steps(&mut self, n: u64) {
        self.count += n;
        if let Some(el) = &self.element {
            el.set_text_content(Some(&format!("Steps: {}", self.count)));
        }
    }

    pub fn reset(&mut self) {
        self.count = 1;
        if let Some(el) = &self.element {
            el.set_text_content(Some("Steps: 0"));
        }
    }

    pub fn get_count(&self) -> u64 {
        self.count
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

fn url() -> url::Url {
    document().url().unwrap().parse().unwrap()
}

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

fn add_debug_url_param() {
    use web_sys::wasm_bindgen::JsValue;

    let mut new_url = url();
    let mut params: HashMap<String, String> = new_url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    params.insert("debug".into(), String::new());
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

fn remove_url_param(key: &str) {
    use web_sys::wasm_bindgen::JsValue;

    let mut new_url = url();
    let mut params: HashMap<String, String> = new_url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    // remove old parameter
    params.retain(|k, _| k != key);
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

fn remove_all_url_params_except(key: &str) {
    use web_sys::wasm_bindgen::JsValue;

    let mut new_url = url();
    let mut params: HashMap<String, String> = new_url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    // remove old parameters
    params.retain(|k, _| k == key);
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
    fn register_shortcut(state: Rc<RefCell<DebugUIState>>) -> EventListener {
        let doc = document();
        EventListener::new(&doc, "keydown", move |event| {
            let Some(key_event) = event.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            if key_event.shift_key() && key_event.key() == "I" {
                let (was_enabled, root, next_uid) = {
                    let s = state.borrow();
                    match &*s {
                        DebugUIState::Enabled { root, next_uid, .. } => {
                            (true, root.clone(), *next_uid)
                        }
                        DebugUIState::Disabled { root, next_uid } => {
                            (false, root.clone(), *next_uid)
                        }
                    }
                };
                let new_state = if was_enabled {
                    remove_url_param("debug");
                    root.set_attribute("style", "display: none").unwrap();
                    DebugUIState::Disabled { root, next_uid }
                } else {
                    add_debug_url_param();
                    root.remove_attribute("style").unwrap();
                    DebugUIState::Enabled {
                        root,
                        next_uid,
                        needs_restart: false,
                    }
                };
                *state.borrow_mut() = new_state;
            }
        })
    }

    pub fn new(title: impl AsRef<str>) -> Self {
        let document = document();
        let title = title.as_ref().to_owned();
        let debug_enabled = url().query_pairs().any(|param| param.0 == "debug");
        let needs_clear_shared = Rc::new(RefCell::new(false));

        // Create state placeholder before enable() so event handlers can reference it
        let state = Rc::new(RefCell::new(DebugUIState::Disabled {
            root: document.create_element("div").unwrap(),
            next_uid: 0,
        }));

        let initial_state =
            match Self::enable(&title, needs_clear_shared.clone(), Some(state.clone())) {
                DebugUIState::Enabled { root, next_uid, .. } if !debug_enabled => {
                    root.set_attribute("style", "display: none").unwrap();
                    DebugUIState::Disabled { root, next_uid }
                }
                s => s,
            };
        *state.borrow_mut() = initial_state;

        let shortcut_listener = Self::register_shortcut(state.clone());
        Self {
            state,
            _shortcut_listener: shortcut_listener,
            document,
            needs_clear_shared,
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(*self.state.borrow(), DebugUIState::Enabled { .. })
    }

    pub fn start_section<S: AsRef<str>>(&mut self, title: S) {
        let state = self.state.borrow();
        let root = match &*state {
            DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => root,
        };
        let document = document();
        let el = document.create_element("h3").unwrap();
        el.set_text_content(Some(title.as_ref()));
        el.set_class_name("DebugUI-section-title");
        root.append_child(&el).unwrap();
    }

    pub fn param<
        T: Copy + ToString + FromStr + ToPrimitive + FromPrimitive + 'static,
        S: AsRef<str> + Clone,
    >(
        &mut self,
        p: ParamParam<T, S>,
    ) -> Param<T> {
        let key = p.name.as_ref().replace(" ", "_");
        let default_value = url()
            .query_pairs()
            .find(|(k, _)| k.as_ref() == key)
            .map(|(_, v)| v.parse())
            .into_iter()
            .flatten()
            .next()
            .unwrap_or(p.default_value);

        let (send, param_value) = Param::new(default_value);
        let doc = self.document.clone();
        let state = self.state.clone();
        let mut state_match = state.borrow_mut();
        match &mut *state_match {
            DebugUIState::Enabled { root, next_uid, .. }
            | DebugUIState::Disabled { root, next_uid } => {
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
                            p.range.start().to_f64().unwrap(),
                            p.range.end().to_f64().unwrap(),
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
                    let state = state.clone();
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
                        let value = T::from_f64(scaled).unwrap_or_else(|| {
                            panic!("Failed to cast slider value for parameter {name}")
                        });

                        value_input.set_value_as_number(value.to_f64().unwrap());

                        add_url_param(&key, value);

                        send.send(value).unwrap();
                        if p.needs_restart {
                            Self::set_needs_restart(&state);
                        }
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
                    let state = state.clone();
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

                        add_url_param(&key, value);

                        send.send(value).unwrap();
                        if p.needs_restart {
                            Self::set_needs_restart(&state);
                        }
                    })
                    .forget();
                }
            }
        }
        param_value
    }

    fn set_needs_restart(state: &Rc<RefCell<DebugUIState>>) {
        if let DebugUIState::Enabled { needs_restart, .. } = &mut *state.borrow_mut() {
            *needs_restart = true;
        }
    }

    pub fn link(&mut self, text: &str, href: &str) {
        let state = self.state.borrow();
        let root = match &*state {
            DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => root,
        };
        let a = self.document.create_element("a").unwrap();
        a.set_text_content(Some(text));
        a.set_attribute("href", href).unwrap();
        a.set_attribute("target", "_blank").unwrap();
        a.set_class_name("DebugUI-link");
        root.append_child(&a).unwrap();
    }
    pub fn should_restart(&mut self) -> bool {
        let mut state = self.state.borrow_mut();
        match &mut *state {
            DebugUIState::Enabled { needs_restart, .. } => {
                let result = *needs_restart;
                *needs_restart = false;
                result
            }
            DebugUIState::Disabled { .. } => false,
        }
    }

    pub fn needs_clear(&self) -> Rc<RefCell<bool>> {
        self.needs_clear_shared.clone()
    }

    pub fn step_counter(&mut self) -> StepCounter {
        match &*self.state.borrow() {
            DebugUIState::Enabled { root, .. } => {
                let doc = document();
                let el = doc.create_element("div").unwrap();
                el.set_class_name("DebugUI-step-counter");
                el.set_text_content(Some("Steps: 0"));
                root.append_child(&el).unwrap();
                StepCounter {
                    element: Some(el),
                    count: 1,
                }
            }
            DebugUIState::Disabled { .. } => StepCounter {
                element: None,
                count: 0,
            },
        }
    }
    fn enable(
        title: impl AsRef<str>,
        needs_clear: Rc<RefCell<bool>>,
        state: Option<Rc<RefCell<DebugUIState>>>,
    ) -> DebugUIState {
        let document = document();
        let body = document.body().expect("document should have a body");
        let root = document.create_element("div").unwrap();
        let title_line = document.create_element("div").unwrap();
        let title_elt = document.create_element("h2").unwrap();
        let close_btn = document.create_element("button").unwrap();
        let reset_btn = document.create_element("button").unwrap();
        let clear_btn = document.create_element("button").unwrap();

        title_elt.set_text_content(Some(title.as_ref()));
        close_btn.set_text_content(Some("🗙"));
        reset_btn.set_text_content(Some("Reset params"));
        clear_btn.set_text_content(Some("Clear canvas"));

        root.set_class_name("DebugUI-root-box");
        title_elt.set_class_name("DebugUI-title");
        title_line.set_class_name("DebugUI-title-line");
        close_btn.set_class_name("DebugUI-close-btn");
        reset_btn.set_class_name("DebugUI-reset-btn");
        clear_btn.set_class_name("DebugUI-clear-btn");

        title_line.append_child(&title_elt).unwrap();
        title_line.append_child(&close_btn).unwrap();
        root.append_child(&title_line).unwrap();
        root.append_child(&reset_btn).unwrap();
        root.append_child(&clear_btn).unwrap();
        body.append_child(&root).unwrap();

        let style = document.create_element("style").unwrap();
        style.set_text_content(Some(include_str!("./style.css")));
        document.head().unwrap().append_child(&style).unwrap();

        {
            let root = root.clone();
            let state = state.clone();
            EventListener::new(&close_btn, "click", move |_event| {
                remove_url_param("debug");
                root.set_attribute("style", "display: none").unwrap();
                if let Some(ref s) = state {
                    let mut s = s.borrow_mut();
                    if let DebugUIState::Enabled {
                        root: r, next_uid, ..
                    } = &*s
                    {
                        *s = DebugUIState::Disabled {
                            root: r.clone(),
                            next_uid: *next_uid,
                        };
                    }
                }
            })
            .forget();
        }
        {
            EventListener::new(&reset_btn, "click", move |_event| {
                remove_all_url_params_except("debug");
                window().location().reload().unwrap();
            })
            .forget();
        }
        {
            let needs_clear = needs_clear.clone();
            EventListener::new(&clear_btn, "click", move |_event| {
                *needs_clear.borrow_mut() = true;
            })
            .forget();
        }

        DebugUIState::Enabled {
            root,
            next_uid: 0,
            needs_restart: false,
        }
    }
}

impl Drop for DebugUI {
    fn drop(&mut self) {
        let root = match &*self.state.borrow() {
            DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => {
                root.clone()
            }
        };
        root.remove();
    }
}

impl Scale {
    // these doc strings are only true for Logarithmic scale smh..

    /// - input: a float in the range 0..1
    /// - min: minimum output value
    /// - max: maximum output value
    fn scale<T: ToPrimitive>(self, input: f64, range: &RangeInclusive<T>) -> f64 {
        match self {
            Scale::Linear => input,
            Scale::Logarithmic => {
                (input
                    * (range.end().to_f64().unwrap() - range.start().to_f64().unwrap() + 1.).ln())
                .exp()
                    + range.start().to_f64().unwrap()
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
    fn unscale<T1: ToPrimitive, T2: ToPrimitive>(
        self,
        input: T2,
        range: &RangeInclusive<T1>,
    ) -> f64 {
        match self {
            Scale::Linear => input.to_f64().unwrap(),
            Scale::Logarithmic => {
                (input.to_f64().unwrap() - range.start().to_f64().unwrap() + 1.).ln()
                    / (range.end().to_f64().unwrap() - range.start().to_f64().unwrap() + 1.).ln()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Scale, StepCounter};
    use rstest::rstest;

    #[test]
    fn step_counter_add_steps() {
        let mut counter = StepCounter {
            element: None,
            count: 0,
        };
        counter.add_steps(5);
        assert_eq!(counter.get_count(), 5);
        counter.add_steps(3);
        assert_eq!(counter.get_count(), 8);
    }

    #[test]
    fn step_counter_reset() {
        let mut counter = StepCounter {
            element: None,
            count: 42,
        };
        counter.reset();
        assert_eq!(counter.get_count(), 1);
    }

    #[test]
    fn step_counter_add_zero() {
        let mut counter = StepCounter {
            element: None,
            count: 10,
        };
        counter.add_steps(0);
        assert_eq!(counter.get_count(), 10);
    }

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
        let scaled = scale.scale(input, &(min..=max));
        let unscaled = scale.unscale(output, &(min..=max));
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
