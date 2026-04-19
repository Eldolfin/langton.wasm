use gloo::events::EventListener;
use num_traits::{FromPrimitive, Num, ToPrimitive};
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::RangeInclusive,
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
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
    inner: Arc<RwLock<T>>,
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
    fn new(value: T) -> (Arc<RwLock<T>>, Self) {
        let inner = Arc::new(RwLock::new(value));
        (Arc::clone(&inner), Self { inner })
    }

    pub fn fixed(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    pub fn get(&self) -> T {
        *self.inner.read().unwrap()
    }
}

impl<T: Copy> Clone for Param<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub struct StepCounter {
    element: Option<Element>,
    count: u64,
}

impl StepCounter {
    pub fn disabled() -> Self {
        Self {
            element: None,
            count: 0,
        }
    }

    pub fn add_steps(&mut self, n: u64) {
        self.count += n;
        if let Some(el) = &self.element {
            el.set_text_content(Some(&format!("Steps: {}", self.count)));
        }
    }

    pub fn reset(&mut self) {
        self.count = 0;
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

thread_local! {
    static HISTORY_PUSHED: RefCell<bool> = const { RefCell::new(false) };
}

fn push_or_replace_url(new_url: &str) {
    use web_sys::wasm_bindgen::JsValue;
    let history = window().history().unwrap();
    HISTORY_PUSHED.with(|pushed| {
        if *pushed.borrow() {
            history
                .replace_state_with_url(&JsValue::NULL, "", Some(new_url))
                .unwrap();
        } else {
            history
                .push_state_with_url(&JsValue::NULL, "", Some(new_url))
                .unwrap();
            *pushed.borrow_mut() = true;
        }
    });
}

fn modify_url_params(f: impl FnOnce(&mut HashMap<String, String>)) {
    let mut new_url = url();
    let mut params: HashMap<String, String> = new_url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    f(&mut params);
    new_url.query_pairs_mut().clear();
    let mut params: Vec<_> = params.into_iter().collect();
    params.sort();
    new_url.query_pairs_mut().extend_pairs(params);
    push_or_replace_url(new_url.as_str());
}

fn add_url_param<T: Copy + ToString + FromStr + ToPrimitive + FromPrimitive + 'static>(
    key: &str,
    value: T,
) {
    modify_url_params(|params| {
        params.retain(|k, _| k != key);
        params.insert(key.into(), value.to_string());
    });
}

fn add_debug_url_param() {
    modify_url_params(|params| {
        params.insert("debug".into(), String::new());
    });
}

fn remove_url_param(key: &str) {
    modify_url_params(|params| {
        params.retain(|k, _| k != key);
    });
}

fn remove_all_url_params_except(keys: &[&str]) {
    modify_url_params(|params| {
        params.retain(|k, _| keys.contains(&k.as_str()));
    });
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

    /// Headless instance: no DOM elements created. For use in previews and tests.
    pub fn headless() -> Self {
        let document = document();
        let state = Rc::new(RefCell::new(DebugUIState::Disabled {
            root: document.create_element("div").unwrap(),
            next_uid: 0,
        }));
        let shortcut_listener = Self::register_shortcut(state.clone());
        Self {
            state,
            _shortcut_listener: shortcut_listener,
            document,
            needs_clear_shared: Rc::new(RefCell::new(false)),
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

        let (writer, param_value) = Param::new(default_value);
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
                    let writer = Arc::clone(&writer);
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

                        *writer.write().unwrap() = value;
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
                    let writer = Arc::clone(&writer);
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

                        *writer.write().unwrap() = value;
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

    pub fn presets(&mut self, presets: &[(&'static str, &'static str)]) {
        let state = self.state.borrow();
        let root = match &*state {
            DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => root,
        };
        let document = document();
        let select = document.create_element("select").unwrap();
        select.set_class_name("DebugUI-presets-select");

        let placeholder = document.create_element("option").unwrap();
        placeholder.set_text_content(Some("— Presets —"));
        placeholder.set_attribute("disabled", "").unwrap();
        placeholder.set_attribute("selected", "").unwrap();
        select.append_child(&placeholder).unwrap();

        for (name, query_string) in presets {
            let option = document.create_element("option").unwrap();
            option.set_text_content(Some(name));
            option.set_attribute("value", query_string).unwrap();
            select.append_child(&option).unwrap();
        }

        {
            use web_sys::HtmlSelectElement;
            let select_clone = select.clone().dyn_into::<HtmlSelectElement>().unwrap();
            EventListener::new(&select, "change", move |_event| {
                let value = select_clone.value();
                let mut new_url = url();
                // Keep only animation and debug params
                let kept: Vec<(String, String)> = new_url
                    .query_pairs()
                    .filter(|(k, _)| k == "animation" || k == "debug")
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                // Parse preset query string params
                let preset_params: Vec<(String, String)> = value
                    .split('&')
                    .filter_map(|pair: &str| {
                        let mut parts = pair.splitn(2, '=');
                        let k = parts.next()?;
                        let v = parts.next().unwrap_or("");
                        if k.is_empty() {
                            None
                        } else {
                            Some((k.to_string(), v.to_string()))
                        }
                    })
                    .collect();
                new_url.query_pairs_mut().clear();
                let mut all_params: Vec<(String, String)> = kept;
                all_params.extend(preset_params);
                all_params.sort();
                new_url.query_pairs_mut().extend_pairs(&all_params);
                window().location().assign(new_url.as_str()).unwrap();
            })
            .forget();
        }

        // Insert before reset_btn
        let reset_btn = root.query_selector(".DebugUI-reset-btn").unwrap();
        root.insert_before(&select, reset_btn.as_ref().map(|e| e as &web_sys::Node))
            .unwrap();
    }

    pub fn link(&mut self, text: &str, href: &str) {
        let a = self.document.create_element("a").unwrap();
        a.set_text_content(Some(text));
        a.set_attribute("href", href).unwrap();
        a.set_attribute("target", "_blank").unwrap();
        a.set_class_name("DebugUI-link");
        self.root().append_child(&a).unwrap();
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
                    count: 0,
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
                remove_all_url_params_except(&["debug", "animation"]);
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

    pub fn add_footer(&mut self) {
        self.link(
            "About this animation",
            "https://codeberg.org/eldolfin/langton.wasm",
        );
        self.ai_impl_dropdown();
    }

    pub fn ai_impl_dropdown(&mut self) {
        const PROMPT: &str = include_str!("../../../prompts/FETCH-APPLY-CHANGES.md");

        const CHATGPT_SVG: &str = r#"<svg width="26" height="26" viewBox="0 0 41 41" fill="currentColor"><path d="M37.532 16.87a9.963 9.963 0 0 0-.856-8.184 10.078 10.078 0 0 0-10.855-4.835 9.964 9.964 0 0 0-6.212-2.854A10.079 10.079 0 0 0 9.116 5.97a9.964 9.964 0 0 0-6.664 4.834 10.079 10.079 0 0 0 1.24 11.817 9.965 9.965 0 0 0 .856 8.185 10.079 10.079 0 0 0 10.855 4.835 9.965 9.965 0 0 0 6.212 2.854 10.079 10.079 0 0 0 10.514-4.963 9.964 9.964 0 0 0 6.663-4.834 10.079 10.079 0 0 0-1.24-11.817zM22.498 37.886a7.474 7.474 0 0 1-4.799-1.735c.061-.033.168-.091.237-.134l7.964-4.6a1.294 1.294 0 0 0 .655-1.134V19.054l3.366 1.944a.12.12 0 0 1 .066.092v9.299a7.505 7.505 0 0 1-7.49 7.496zM6.392 31.006a7.471 7.471 0 0 1-.894-5.023c.06.036.162.099.237.141l7.964 4.6a1.297 1.297 0 0 0 1.308 0l9.724-5.614v3.888a.12.12 0 0 1-.048.103l-8.051 4.649a7.504 7.504 0 0 1-10.24-2.744zM4.297 13.62A7.469 7.469 0 0 1 8.2 10.333c0 .068-.004.19-.004.274v9.201a1.294 1.294 0 0 0 .654 1.132l9.723 5.614-3.366 1.944a.12.12 0 0 1-.114.012L6.044 23.86a7.504 7.504 0 0 1-1.747-10.24zm27.658 6.437l-9.724-5.615 3.367-1.943a.121.121 0 0 1 .114-.012l9.048 5.228a7.498 7.498 0 0 1-1.158 13.528v-9.476a1.293 1.293 0 0 0-.647-1.71zm3.35-5.043c-.059-.037-.162-.099-.236-.141l-7.965-4.6a1.298 1.298 0 0 0-1.308 0l-9.723 5.614v-3.888a.12.12 0 0 1 .048-.103l8.05-4.645a7.497 7.497 0 0 1 11.135 7.763zm-21.063 6.929l-3.367-1.944a.12.12 0 0 1-.065-.092v-9.299a7.497 7.497 0 0 1 12.293-5.756 6.94 6.94 0 0 0-.236.134l-7.965 4.6a1.294 1.294 0 0 0-.654 1.132l-.006 11.225zm1.829-3.943l4.33-2.501 4.332 2.498v4.993l-4.331 2.5-4.331-2.5V18z"/></svg>"#;
        const CLAUDE_SVG: &str = r#"<svg width="26" height="26" viewBox="0 0 24 24" fill="currentColor"><path d="M13.827 3.52h3.603L24 20.32h-3.603l-6.57-16.8zm-3.654 0H6.57L0 20.32h3.603l1.498-3.87h6.89l1.498 3.87h3.604l-6.57-16.8h-.15zm-1.177 9.836 2.274-5.89 2.275 5.89H8.996z"/></svg>"#;
        const GEMINI_SVG: &str = r##"<svg width="26" height="26" viewBox="0 0 28 28" fill="none"><defs><linearGradient id="gg" x1="0" y1="0" x2="28" y2="28" gradientUnits="userSpaceOnUse"><stop offset="0%" stop-color="#4285F4"/><stop offset="50%" stop-color="#9B72CB"/><stop offset="100%" stop-color="#EA4335"/></linearGradient></defs><path d="M14 28C14 26.0633 13.6267 24.2433 12.88 22.54C12.1567 20.8367 11.165 19.355 9.905 18.095C8.645 16.835 7.16333 15.8433 5.46 15.12C3.75667 14.3733 1.93667 14 0 14C1.93667 14 3.75667 13.6383 5.46 12.915C7.16333 12.1683 8.645 11.165 9.905 9.905C11.165 8.645 12.1567 7.16333 12.88 5.46C13.6267 3.75667 14 1.93667 14 0C14 1.93667 14.3617 3.75667 15.085 5.46C15.8317 7.16333 16.835 8.645 18.095 9.905C19.355 11.165 20.8367 12.1683 22.54 12.915C24.2433 13.6383 26.0633 14 28 14C26.0633 14 24.2433 14.3733 22.54 15.12C20.8367 15.8433 19.355 16.835 18.095 18.095C16.835 19.355 15.8317 20.8367 15.085 22.54C14.3617 24.2433 14 26.0633 14 28Z" fill="url(#gg)"/></svg>"##;
        const COPY_SVG: &str = r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>"#;
        const CHECK_SVG: &str = r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>"#;

        let doc = self.document.clone();
        let root = self.root().clone();

        const SPARKLE_SVG: &str = r##"<svg width="15" height="15" viewBox="0 0 20 20" fill="none" style="flex-shrink:0"><path d="M10 1C10 1 11.2 7 16 8C11.2 9 10 15 10 15C10 15 8.8 9 4 8C8.8 7 10 1 10 1Z" fill="#c4b5fd"/><path d="M16.5 12.5C16.5 12.5 17.1 14.9 19 15.5C17.1 16.1 16.5 18.5 16.5 18.5C16.5 18.5 15.9 16.1 14 15.5C15.9 14.9 16.5 12.5 16.5 12.5Z" fill="#c4b5fd" opacity="0.75"/><path d="M4 1.5C4 1.5 4.5 3.5 6 4C4.5 4.5 4 6.5 4 6.5C4 6.5 3.5 4.5 2 4C3.5 3.5 4 1.5 4 1.5Z" fill="#c4b5fd" opacity="0.6"/></svg>Bugfix / new feature idea ?"##;

        let wrapper = doc.create_element("div").unwrap();
        wrapper.set_class_name("DebugUI-ai-launcher");

        let trigger_wrap = doc.create_element("div").unwrap();
        trigger_wrap.set_class_name("DebugUI-ai-trigger-wrap");

        let halo = doc.create_element("div").unwrap();
        halo.set_class_name("DebugUI-ai-halo");
        trigger_wrap.append_child(&halo).unwrap();

        let trigger = doc.create_element("button").unwrap();
        trigger.set_inner_html(SPARKLE_SVG);
        trigger.set_class_name("DebugUI-ai-trigger");
        trigger_wrap.append_child(&trigger).unwrap();
        wrapper.append_child(&trigger_wrap).unwrap();

        let menu = doc.create_element("div").unwrap();
        menu.set_class_name("DebugUI-ai-menu");

        let make_btn = |doc: &Document, svg: &str, color: &str, glow: &str| -> Element {
            let btn = doc.create_element("button").unwrap();
            btn.set_inner_html(svg);
            btn.set_class_name("DebugUI-ai-btn");
            btn.set_attribute("style", &format!("--ai-color: {color}; --ai-glow: {glow}"))
                .unwrap();
            btn
        };

        // ChatGPT
        let chatgpt_btn = make_btn(&doc, CHATGPT_SVG, "#10a37f", "rgba(16,163,127,0.1)");
        {
            let prompt = PROMPT.to_owned();
            EventListener::new(&chatgpt_btn, "click", move |_| {
                let mut u = url::Url::parse("https://chat.openai.com/").unwrap();
                u.query_pairs_mut().append_pair("q", &prompt);
                let _ = window().open_with_url_and_target(u.as_str(), "_blank");
            })
            .forget();
        }
        menu.append_child(&chatgpt_btn).unwrap();

        // Claude
        let claude_btn = make_btn(&doc, CLAUDE_SVG, "#d4702a", "rgba(212,112,42,0.1)");
        {
            let prompt = PROMPT.to_owned();
            EventListener::new(&claude_btn, "click", move |_| {
                let mut u = url::Url::parse("https://claude.ai/new").unwrap();
                u.query_pairs_mut().append_pair("q", &prompt);
                let _ = window().open_with_url_and_target(u.as_str(), "_blank");
            })
            .forget();
        }
        menu.append_child(&claude_btn).unwrap();

        // Gemini
        let gemini_btn = make_btn(&doc, GEMINI_SVG, "#4285F4", "rgba(66,133,244,0.1)");
        {
            let prompt = PROMPT.to_owned();
            EventListener::new(&gemini_btn, "click", move |_| {
                let mut u = url::Url::parse("https://gemini.google.com/guided-learning").unwrap();
                u.query_pairs_mut().append_pair("query", &prompt);
                let _ = window().open_with_url_and_target(u.as_str(), "_blank");
            })
            .forget();
        }
        menu.append_child(&gemini_btn).unwrap();

        // Copy
        let copy_btn = make_btn(&doc, COPY_SVG, "#888", "rgba(160,160,180,0.1)");
        {
            let copy_btn_clone = copy_btn.clone();
            EventListener::new(&copy_btn, "click", move |_| {
                let _ = window().navigator().clipboard().write_text(PROMPT);
                copy_btn_clone.set_inner_html(CHECK_SVG);
                let btn = copy_btn_clone.clone();
                gloo::timers::callback::Timeout::new(2000, move || {
                    btn.set_inner_html(COPY_SVG);
                })
                .forget();
            })
            .forget();
        }
        menu.append_child(&copy_btn).unwrap();

        wrapper.append_child(&menu).unwrap();
        root.append_child(&wrapper).unwrap();
    }

    fn root(&self) -> Element {
        match &*self.state.borrow() {
            DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => {
                root.clone()
            }
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
        assert_eq!(counter.get_count(), 0);
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
    // speed param range: slider endpoints hit the exact bounds
    #[case(Scale::Logarithmic, 0., 0.05, 1_000_000., 0.05)]
    #[case(Scale::Logarithmic, 1., 0.05, 1_000_000., 1_000_000.)]
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
