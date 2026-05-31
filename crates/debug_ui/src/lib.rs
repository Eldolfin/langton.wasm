use common::document;
use common::window;
use gloo::events::EventListener;
use js_sys::Date;
use num_traits::{FromPrimitive, Num, ToPrimitive};
use std::collections::HashMap;
use std::{
    cell::RefCell,
    ops::RangeInclusive,
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
};
pub use web_sys;
use web_sys::{
    Blob, BlobEvent, BlobPropertyBag, Document, Element, HtmlAnchorElement, HtmlInputElement,
    KeyboardEvent, MediaRecorder, MediaRecorderOptions, Url, wasm_bindgen::JsCast as _,
};

const URL_TAG_DEBUG: &str = "debug";
const URL_TAG_ANIMATION: &str = "animation";
const DEBUG_UI_URL_TAGS: &[&str] = &[URL_TAG_DEBUG, URL_TAG_ANIMATION];

struct RecorderState {
    recorder: MediaRecorder,
    _data_listener: EventListener,
    _stop_listener: EventListener,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DebugColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl DebugColor {
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(DebugColor { r, g, b })
    }
}

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        $crate::web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

#[macro_export]
macro_rules! warn {
    ( $( $t:tt )* ) => {
        web_sys::console::warn_1(&format!( $( $t )* ).into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartMode {
    Reload,
    Full,
}

pub enum DebugUIState {
    Enabled {
        root: Element,
        next_uid: u32,
        restart_mode: Option<RestartMode>,
    },
    Disabled {
        root: Element,
        next_uid: u32,
        restart_mode: Option<RestartMode>,
    },
}

impl DebugUIState {
    fn set_restart_mode(&mut self, mode: RestartMode) {
        match self {
            DebugUIState::Enabled { restart_mode, .. } => *restart_mode = Some(mode),
            DebugUIState::Disabled { restart_mode, .. } => *restart_mode = Some(mode),
        }
    }

    fn take_restart_mode(&mut self) -> Option<RestartMode> {
        match self {
            DebugUIState::Enabled { restart_mode, .. } => restart_mode.take(),
            DebugUIState::Disabled { restart_mode, .. } => restart_mode.take(),
        }
    }
}

pub struct DebugUI {
    state: Rc<RefCell<DebugUIState>>,
    _shortcut_listener: EventListener,
    _recorder: Rc<RefCell<Option<RecorderState>>>,
    _stopping_recorder: Rc<RefCell<Option<RecorderState>>>,
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

thread_local! {
    static HISTORY_PUSHED: RefCell<bool> = const { RefCell::new(false) };
}

fn push_or_replace_url(new_url: &str) {
    use common::window;
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
    let mut new_url = common::url();
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
fn add_url_param_empty(key: &str) {
    modify_url_params(|params| {
        params.retain(|k, _| k != key);
        params.insert(key.into(), String::new());
    });
}

fn add_debug_url_param() {
    add_url_param_empty(URL_TAG_DEBUG);
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
    fn register_shortcut(
        state: Rc<RefCell<DebugUIState>>,
        recorder: Rc<RefCell<Option<RecorderState>>>,
        stopping_recorder: Rc<RefCell<Option<RecorderState>>>,
    ) -> EventListener {
        let doc = document();
        let state_captured = state.clone();
        let recorder_captured = recorder.clone();
        let stopping_recorder_captured = stopping_recorder.clone();

        EventListener::new(&doc, "keydown", move |event| {
            let Some(key_event) = event.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            if key_event.shift_key() && key_event.key() == "I" {
                let (was_enabled, root, next_uid) = {
                    let s = state_captured.borrow();
                    match &*s {
                        DebugUIState::Enabled { root, next_uid, .. }
                        | DebugUIState::Disabled { root, next_uid, .. } => (
                            matches!(&*s, DebugUIState::Enabled { .. }),
                            root.clone(),
                            *next_uid,
                        ),
                    }
                };
                let new_state = if was_enabled {
                    remove_url_param(URL_TAG_DEBUG);
                    root.set_attribute("style", "display: none").unwrap();
                    DebugUIState::Disabled {
                        root,
                        next_uid,
                        restart_mode: None,
                    }
                } else {
                    add_debug_url_param();
                    root.remove_attribute("style").unwrap();
                    DebugUIState::Enabled {
                        root,
                        next_uid,
                        restart_mode: None,
                    }
                };
                *state_captured.borrow_mut() = new_state;
            }

            if key_event.shift_key() && key_event.key() == "R" {
                let mut recorder_state = recorder_captured.borrow_mut();
                if let Some(state_to_stop) = recorder_state.take() {
                    state_to_stop.recorder.stop().unwrap();
                    // Move to stopping_recorder to keep listeners alive until onstop fires
                    *stopping_recorder_captured.borrow_mut() = Some(state_to_stop);
                } else {
                    let _doc = document();
                    let _parent = common::get_canvas_parent().expect("canvas parent should exist");
                    let _ = _parent.request_fullscreen();

                    remove_url_param(URL_TAG_DEBUG);
                    let (root, next_uid) = {
                        let s = state_captured.borrow();
                        match &*s {
                            DebugUIState::Enabled { root, next_uid, .. }
                            | DebugUIState::Disabled { root, next_uid, .. } => {
                                (root.clone(), *next_uid)
                            }
                        }
                    };
                    root.set_attribute("style", "display: none").unwrap();
                    *state_captured.borrow_mut() = DebugUIState::Disabled {
                        root,
                        next_uid,
                        restart_mode: None,
                    };

                    // Restart with delay to let resize settle
                    let state_clone = state_captured.clone();
                    gloo::timers::callback::Timeout::new(200, move || {
                        state_clone.borrow_mut().set_restart_mode(RestartMode::Full);
                    })
                    .forget();

                    // Delay recording start so the Full restart has time to recreate the Canvas!
                    let recorder_clone = recorder_captured.clone();
                    let stopping_recorder_clone = stopping_recorder_captured.clone();
                    gloo::timers::callback::Timeout::new(600, move || {
                        let doc = document();
                        let _parent = common::get_canvas_parent().unwrap();
                        let canvas_el = doc.query_selector("canvas").unwrap().unwrap();
                        let canvas = canvas_el.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
                        let stream = match canvas.capture_stream() {
                            Ok(s) => s,
                            Err(_) => {
                                return;
                            }
                        };

                        let options = MediaRecorderOptions::new();
                        options.set_video_bits_per_second(8_000_000);
                        options.set_mime_type("video/webm;codecs=vp9");

                        let recorder_inst =
                            match MediaRecorder::new_with_media_stream_and_media_recorder_options(
                                &stream, &options,
                            ) {
                                Ok(r) => r,
                                Err(_) => {
                                    // Fallback to default options if VP9 is not supported
                                    MediaRecorder::new_with_media_stream(&stream).unwrap()
                                }
                            };

                        let chunks = Rc::new(RefCell::new(Vec::new()));
                        let data_chunks = chunks.clone();
                        let data_listener =
                            EventListener::new(&recorder_inst, "dataavailable", move |e| {
                                let event = e.dyn_ref::<BlobEvent>().unwrap();
                                data_chunks.borrow_mut().push(event.data().unwrap());
                            });

                        let stop_chunks = chunks.clone();
                        let stopping_recorder_done = stopping_recorder_clone.clone();
                        let stop_listener = EventListener::new(&recorder_inst, "stop", move |_e| {
                            let array = js_sys::Array::new();
                            for chunk in stop_chunks.borrow().iter() {
                                array.push(chunk);
                            }
                            let options = BlobPropertyBag::new();
                            let blob =
                                Blob::new_with_blob_sequence_and_options(&array, &options).unwrap();
                            let url = Url::create_object_url_with_blob(&blob).unwrap();

                            let doc = document();
                            let a = doc
                                .create_element("a")
                                .unwrap()
                                .dyn_into::<HtmlAnchorElement>()
                                .unwrap();
                            a.set_href(&url);
                            let animation_name =
                                get_url_param("animation").unwrap_or("menu".to_owned());
                            let now = Date::new_0().to_iso_string();
                            let file_name = format!("recording-{animation_name}-{now}.webm");
                            a.set_download(&file_name);
                            a.click();

                            let url_to_revoke = url.clone();
                            gloo::timers::callback::Timeout::new(1000, move || {
                                let _ = Url::revoke_object_url(&url_to_revoke);
                            })
                            .forget();

                            // Clear stopping state
                            *stopping_recorder_done.borrow_mut() = None;
                        });

                        if recorder_inst.start().is_err() {
                            return;
                        }

                        *recorder_clone.borrow_mut() = Some(RecorderState {
                            recorder: recorder_inst,
                            _data_listener: data_listener,
                            _stop_listener: stop_listener,
                        });
                    })
                    .forget();
                }
            }
        })
    }

    pub fn new(title: impl AsRef<str>) -> Self {
        {
            let document = document();
            let title = title.as_ref().to_owned();
            let debug_enabled = has_url_tag(URL_TAG_DEBUG);
            let needs_clear_shared = Rc::new(RefCell::new(false));

            // Create state placeholder before enable() so event handlers can reference it
            let state = Rc::new(RefCell::new(DebugUIState::Disabled {
                root: document.create_element("div").unwrap(),
                next_uid: 0,
                restart_mode: None,
            }));
            let recorder = Rc::new(RefCell::new(None));
            let stopping_recorder = Rc::new(RefCell::new(None));

            let initial_state =
                match Self::enable(&title, needs_clear_shared.clone(), Some(state.clone())) {
                    DebugUIState::Enabled { root, next_uid, .. } if !debug_enabled => {
                        root.set_attribute("style", "display: none").unwrap();
                        DebugUIState::Disabled {
                            root,
                            next_uid,
                            restart_mode: None,
                        }
                    }
                    s => s,
                };
            *state.borrow_mut() = initial_state;

            let shortcut_listener =
                Self::register_shortcut(state.clone(), recorder.clone(), stopping_recorder.clone());
            Self {
                state,
                _shortcut_listener: shortcut_listener,
                _recorder: recorder,
                _stopping_recorder: stopping_recorder,
                document,
                needs_clear_shared,
            }
        }
    }

    /// Headless instance: no DOM elements created. For use in previews and tests.
    pub fn headless() -> Self {
        {
            let document = document();
            let state = Rc::new(RefCell::new(DebugUIState::Disabled {
                root: document.create_element("div").unwrap(),
                next_uid: 0,
                restart_mode: None,
            }));
            let recorder = Rc::new(RefCell::new(None));
            let stopping_recorder = Rc::new(RefCell::new(None));
            let shortcut_listener =
                Self::register_shortcut(state.clone(), recorder.clone(), stopping_recorder.clone());
            Self {
                state,
                _shortcut_listener: shortcut_listener,
                _recorder: recorder,
                _stopping_recorder: stopping_recorder,
                document,
                needs_clear_shared: Rc::new(RefCell::new(false)),
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(*self.state.borrow(), DebugUIState::Enabled { .. })
    }

    pub fn start_section<S: AsRef<str>>(&mut self, title: S) {
        {
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
    }

    pub fn param<
        T: Copy + ToString + FromStr + ToPrimitive + FromPrimitive + 'static,
        S: AsRef<str> + Clone,
    >(
        &mut self,
        p: ParamParam<T, S>,
    ) -> Param<T> {
        {
            let key = p.name.as_ref().replace(" ", "_");
            let default_value = common::url()
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
                | DebugUIState::Disabled {
                    root,
                    next_uid,
                    restart_mode: _,
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
                                Self::set_restart_mode(&state, RestartMode::Reload);
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
                                Self::set_restart_mode(&state, RestartMode::Reload);
                            }
                        })
                        .forget();
                    }
                }
            }
            param_value
        }
    }

    pub fn color_param(&mut self, name: &str, default: DebugColor) -> Param<DebugColor> {
        {
            let key = name.replace(" ", "_");
            let default_value = common::url()
                .query_pairs()
                .find(|(k, _)| k.as_ref() == key)
                .and_then(|(_, v)| DebugColor::from_hex(v.as_ref()))
                .unwrap_or(default);

            let (writer, param_value) = Param::new(default_value);
            let doc = self.document.clone();
            let state = self.state.clone();
            let mut state_match = state.borrow_mut();
            match &mut *state_match {
                DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => {
                    let container = doc.create_element("div").unwrap();
                    let label = doc.create_element("label").unwrap();
                    let preview = doc.create_element("div").unwrap();
                    let color_input = doc
                        .create_element("input")
                        .unwrap()
                        .dyn_into::<HtmlInputElement>()
                        .unwrap();

                    container.set_class_name("DebugUI-param-container");
                    label.set_class_name("DebugUI-param-label");
                    label.set_text_content(Some(name));
                    preview.set_class_name("DebugUI-color-preview");
                    preview
                        .set_attribute(
                            "style",
                            &format!("background-color: {}", default_value.to_hex()),
                        )
                        .unwrap();
                    color_input.set_attribute("type", "color").unwrap();
                    color_input.set_class_name("DebugUI-color-input");
                    color_input.set_value(&default_value.to_hex());

                    container.append_child(&label).unwrap();
                    container.append_child(&preview).unwrap();
                    container.append_child(&color_input).unwrap();
                    root.append_child(&container).unwrap();

                    // Clicking the preview opens the hidden color input
                    {
                        let color_input_clone = color_input.clone();
                        EventListener::new(&preview, "click", move |_event| {
                            color_input_clone.click();
                        })
                        .forget();
                    }

                    // On color input change, update preview + param + URL
                    {
                        let preview = preview.clone();
                        let writer = Arc::clone(&writer);
                        let key = key.clone();
                        EventListener::new(&color_input, "input", move |event| {
                            let input = event
                                .target()
                                .unwrap()
                                .dyn_into::<HtmlInputElement>()
                                .unwrap();
                            let hex = input.value();
                            if let Some(color) = DebugColor::from_hex(&hex) {
                                preview
                                    .set_attribute(
                                        "style",
                                        &format!("background-color: {}", color.to_hex()),
                                    )
                                    .unwrap();
                                *writer.write().unwrap() = color;
                                let key = key.clone();
                                modify_url_params(|params| {
                                    params.retain(|k, _| k != &key);
                                    params.insert(key.clone(), color.to_hex());
                                });
                            }
                        })
                        .forget();
                    }
                }
            }
            param_value
        }
    }

    fn set_restart_mode(state: &Rc<RefCell<DebugUIState>>, mode: RestartMode) {
        state.borrow_mut().set_restart_mode(mode);
    }

    pub fn presets(&mut self, presets: &[(&'static str, &'static str)]) {
        {
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
                    use common::console_log;

                    let value = select_clone.value();
                    let mut new_url = common::url();
                    // Keep only animation and debug params
                    let kept: Vec<(String, String)> = new_url
                        .query_pairs()
                        .filter(|(k, _)| DEBUG_UI_URL_TAGS.contains(&k.as_ref()))
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    new_url.query_pairs_mut().clear();
                    new_url.query_pairs_mut().extend_pairs(&kept);
                    let url = format!("{}&{}", new_url.as_str(), value);
                    console_log!("{url:?}");
                    window().location().assign(&url).unwrap();
                })
                .forget();
            }

            // Insert before reset_btn
            let reset_btn = root.query_selector(".DebugUI-reset-btn").unwrap();
            root.insert_before(&select, reset_btn.as_ref().map(|e| e as &web_sys::Node))
                .unwrap();
        }
    }

    pub fn link(&mut self, text: &str, href: &str) {
        {
            let a = self.document.create_element("a").unwrap();
            a.set_text_content(Some(text));
            a.set_attribute("href", href).unwrap();
            a.set_attribute("target", "_blank").unwrap();
            a.set_class_name("DebugUI-link");
            self.root().append_child(&a).unwrap();
        }
    }

    pub fn take_restart_mode(&mut self) -> Option<RestartMode> {
        self.state.borrow_mut().take_restart_mode()
    }

    pub fn needs_clear(&self) -> Rc<RefCell<bool>> {
        self.needs_clear_shared.clone()
    }

    pub fn step_counter(&mut self) -> StepCounter {
        {
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
    }
    fn enable(
        title: impl AsRef<str>,
        needs_clear: Rc<RefCell<bool>>,
        state: Option<Rc<RefCell<DebugUIState>>>,
    ) -> DebugUIState {
        use common::get_canvas_parent;

        let document = document();
        let root = document.create_element("div").unwrap();
        let title_line = document.create_element("div").unwrap();
        let title_elt = document.create_element("h2").unwrap();
        let fullscreen_btn = document.create_element("button").unwrap();
        let menu_btn = document.create_element("button").unwrap();
        let close_btn = document.create_element("button").unwrap();
        let reset_btn = document.create_element("button").unwrap();
        let clear_btn = document.create_element("button").unwrap();

        title_elt.set_text_content(Some(title.as_ref()));
        fullscreen_btn.set_text_content(Some("🎦"));
        menu_btn.set_text_content(Some("↩"));
        close_btn.set_text_content(Some("❌"));
        reset_btn.set_text_content(Some("Reset params"));
        clear_btn.set_text_content(Some("Clear canvas"));

        root.set_class_name("DebugUI-root-box");
        title_elt.set_class_name("DebugUI-title");
        title_line.set_class_name("DebugUI-title-line");
        fullscreen_btn.set_class_name("DebugUI-fullscreen-btn");
        fullscreen_btn.set_class_name("DebugUI-menu-btn");
        close_btn.set_class_name("DebugUI-close-btn");
        reset_btn.set_class_name("DebugUI-reset-btn");
        clear_btn.set_class_name("DebugUI-clear-btn");

        title_line.append_child(&title_elt).unwrap();
        title_line.append_child(&fullscreen_btn).unwrap();
        title_line.append_child(&menu_btn).unwrap();
        title_line.append_child(&close_btn).unwrap();
        root.append_child(&title_line).unwrap();
        root.append_child(&reset_btn).unwrap();
        root.append_child(&clear_btn).unwrap();

        let container = get_canvas_parent().unwrap();
        container.append_child(&root).unwrap();

        let style = document.create_element("style").unwrap();
        style.set_text_content(Some(include_str!("./style.css")));
        document.head().unwrap().append_child(&style).unwrap();

        {
            let root = root.clone();
            let state = state.clone();
            let container = container.clone();
            EventListener::new(&fullscreen_btn, "click", move |_event| {
                container.request_fullscreen().unwrap();

                remove_url_param(URL_TAG_DEBUG);
                let (root, next_uid) = if let Some(state) = state.as_ref() {
                    let s = state.borrow();
                    match &*s {
                        DebugUIState::Enabled { root, next_uid, .. }
                        | DebugUIState::Disabled {
                            root,
                            next_uid,
                            restart_mode: _,
                        } => (root.clone(), *next_uid),
                    }
                } else {
                    (root.clone(), 0)
                };
                root.set_attribute("style", "display: none").unwrap();
                if let Some(state) = state.as_ref() {
                    *state.borrow_mut() = DebugUIState::Disabled {
                        root,
                        next_uid,
                        restart_mode: None,
                    };
                }

                let state_clone = state.clone();
                gloo::timers::callback::Timeout::new(2, move || {
                    if let Some(state) = state_clone {
                        state.borrow_mut().set_restart_mode(RestartMode::Full);
                    }
                })
                .forget();
            })
            .forget();
        }
        {
            EventListener::new(&menu_btn, "click", move |_event| {
                modify_url_params(|params| {
                    params.retain(|p, _| p.as_str() == URL_TAG_DEBUG);
                });
                reload();
            })
            .forget();
        }
        {
            let root = root.clone();
            let state = state.clone();
            EventListener::new(&close_btn, "click", move |_event| {
                close_debug_ui(&root, &state);
            })
            .forget();
        }
        {
            EventListener::new(&reset_btn, "click", move |_event| {
                remove_all_url_params_except(DEBUG_UI_URL_TAGS);
                reload();
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
            restart_mode: None,
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
        {
            const PROMPT: &str = include_str!("../../../prompts/FETCH-APPLY-CHANGES.md");

            const CHATGPT_SVG: &str = include_str!("../../../assets/openai-icon-logo.svg");
            const CLAUDE_SVG: &str = include_str!("../../../assets/claude-icon-logo.svg");
            const GEMINI_SVG: &str = include_str!("../../../assets/google-gemini-logo.svg");
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

            const UNAVAILABLE_TOOLTIP: &str =
                "Only Claude is currently powerful enough to execute the needed operations";

            // Claude — first and featured
            let claude_btn = make_btn(&doc, CLAUDE_SVG, "#d4702a", "rgba(212,112,42,0.1)");
            claude_btn
                .set_attribute("class", "DebugUI-ai-btn DebugUI-ai-btn--featured")
                .unwrap();
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

            // ChatGPT
            let chatgpt_btn = make_btn(&doc, CHATGPT_SVG, "#10a37f", "rgba(16,163,127,0.1)");
            chatgpt_btn
                .set_attribute("data-tooltip", UNAVAILABLE_TOOLTIP)
                .unwrap();
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

            // Gemini
            let gemini_btn = make_btn(&doc, GEMINI_SVG, "#4285F4", "rgba(66,133,244,0.1)");
            gemini_btn
                .set_attribute("data-tooltip", UNAVAILABLE_TOOLTIP)
                .unwrap();
            {
                let prompt = PROMPT.to_owned();
                EventListener::new(&gemini_btn, "click", move |_| {
                    let mut u =
                        url::Url::parse("https://gemini.google.com/guided-learning").unwrap();
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
    }

    fn root(&self) -> Element {
        match &*self.state.borrow() {
            DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => {
                root.clone()
            }
        }
    }
}

fn reload() {
    window().location().reload().unwrap();
}

fn has_url_tag(tag: &str) -> bool {
    common::url().query_pairs().any(|param| param.0 == tag)
}

fn get_url_param(tag: &str) -> Option<String> {
    common::url()
        .query_pairs()
        .find(|(param, _)| param == tag)
        .map(|(_, value)| value.into_owned())
}

fn close_debug_ui(root: &Element, state: &Option<Rc<RefCell<DebugUIState>>>) {
    remove_url_param(URL_TAG_DEBUG);
    root.set_attribute("style", "display: none").unwrap();
    if let Some(s) = state {
        let mut s = s.borrow_mut();
        if let DebugUIState::Enabled {
            root: r, next_uid, ..
        } = &*s
        {
            *s = DebugUIState::Disabled {
                root: r.clone(),
                next_uid: *next_uid,
                restart_mode: None,
            };
        }
    }
}

impl Drop for DebugUI {
    fn drop(&mut self) {
        {
            let root = match &*self.state.borrow() {
                DebugUIState::Enabled { root, .. } | DebugUIState::Disabled { root, .. } => {
                    root.clone()
                }
            };
            root.remove();
        }
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
    use super::{DebugColor, Scale, StepCounter};
    use rstest::rstest;

    #[test]
    fn step_counter_add_steps() {
        let mut counter = StepCounter::disabled();
        counter.add_steps(5);
        assert_eq!(counter.get_count(), 5);
        counter.add_steps(3);
        assert_eq!(counter.get_count(), 8);
    }

    #[test]
    fn step_counter_reset() {
        let mut counter = StepCounter::disabled();
        counter.add_steps(42);
        counter.reset();
        assert_eq!(counter.get_count(), 0);
    }

    #[test]
    fn step_counter_add_zero() {
        let mut counter = StepCounter::disabled();
        counter.add_steps(10);
        counter.add_steps(0);
        assert_eq!(counter.get_count(), 10);
    }

    #[test]
    fn debug_color_to_hex() {
        let c = DebugColor {
            r: 255,
            g: 0,
            b: 128,
        };
        assert_eq!(c.to_hex(), "#FF0080");
    }

    #[test]
    fn debug_color_from_hex_with_hash() {
        let c = DebugColor::from_hex("#FF0080").unwrap();
        assert_eq!(
            c,
            DebugColor {
                r: 255,
                g: 0,
                b: 128
            }
        );
    }

    #[test]
    fn debug_color_from_hex_without_hash() {
        let c = DebugColor::from_hex("FF0080").unwrap();
        assert_eq!(
            c,
            DebugColor {
                r: 255,
                g: 0,
                b: 128
            }
        );
    }

    #[test]
    fn debug_color_from_hex_invalid() {
        assert!(DebugColor::from_hex("#GGGGGG").is_none());
        assert!(DebugColor::from_hex("#FFF").is_none());
        assert!(DebugColor::from_hex("").is_none());
    }

    #[test]
    fn debug_color_roundtrip() {
        let original = DebugColor {
            r: 12,
            g: 34,
            b: 56,
        };
        let hex = original.to_hex();
        let recovered = DebugColor::from_hex(&hex).unwrap();
        assert_eq!(original, recovered);
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
