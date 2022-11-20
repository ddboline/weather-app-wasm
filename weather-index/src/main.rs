#![allow(clippy::too_many_lines)]

use dioxus::prelude::{
    dioxus_elements, fc_to_builder, format_args_f, rsx, use_state, Element, LazyNodes, NodeFactory,
    Scope, VNode,
};
use fermi::{use_read, use_set, Atom};
use golde::{call, init_app, trigger, App, Trigger};
use wasm_bindgen::JsValue;
use web_sys::window;

#[cfg(debug_assertions)]
static BASE_URL: &str = "https://www.ddboline.net";

#[cfg(not(debug_assertions))]
static BASE_URL: &str = "";

static DEFAULT_LOCATION: &str = "zip=10001";
static LOCATION: Atom<String> = |_| String::from(DEFAULT_LOCATION);

fn main() {
    // init debug tool for WebAssembly
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();

    dioxus::web::launch(app);
}

fn app(cx: Scope) -> Element {
    let (url_path, set_url_path) = use_state(&cx, || "weather/plot.html").split();
    let (draft, set_draft) = use_state(&cx, String::new).split();
    let (current_loc, set_current_loc) = use_state(&cx, String::new).split();
    let (search_history, set_search_history) = use_state(&cx, || {
        get_history().unwrap_or_else(|_| vec![String::from("zip=10001")])
    })
    .split();

    init_app(&cx, |_| {});

    let location = use_read(&cx, LOCATION);
    let set_location = use_set(&cx, LOCATION);

    let set_location_trigger = set_location.clone();

    let window = web_sys::window().unwrap();
    let search = window.location().search().unwrap();

    if !search.is_empty() && current_loc.is_empty() {
        let s = search.trim_start_matches("?location=");
        let loc = if let Ok(zip) = s.parse::<usize>() {
            format!("zip={zip}")
        } else {
            format!("q={s}")
        };
        if !search_history.contains(&loc) {
            set_search_history.modify(|sh| {
                let mut v: Vec<String> = sh.iter().filter(|x| x.as_str() != loc).cloned().collect();
                v.push(loc.clone());
                set_history(&v).expect("Failed to set history");
                v
            });
            set_search_history.needs_update();
            set_location(loc);
        }
    }

    cx.render(rsx! {
        App {
            trigger: trigger!(
                current_location => move |_, v| {
                    set_location_trigger(v.as_string().unwrap_or_else(|| DEFAULT_LOCATION.into()));
                }
            ),
        }
        body {
            div {
                input {
                    "type": "button",
                    name: "update_location",
                    value: "Update Location",
                    onclick: move |_| {
                        call(&cx, "current_location", "updateLocation();".into());
                    },
                },
                input {
                    "type": "button",
                    name: "text",
                    value: "Text",
                    onclick: move |_| {
                        set_url_path.modify(|_| "weather/index.html");
                    },
                },
                input {
                    "type": "button",
                    name: "plot",
                    value: "Plot",
                    onclick: move |_| {
                        set_url_path.modify(|_| "weather/plot.html");
                    },
                },
                input {
                    "type": "button",
                    name: "wasm",
                    value: "Wasm",
                    onclick: move |_| {
                        set_url_path.modify(|_| "wasm_weather/index.html");
                    },
                },
                form {
                    input {
                        "type": "text",
                        name: "location",
                        value: "{draft}",
                        id: "locationForm",
                        oninput: move |evt| {
                            let msg = evt.value.as_str();
                            set_draft.modify(|_| {msg.into()});
                            set_draft.needs_update();
                        },
                    },
                    input {
                        "type": "button",
                        name: "submitLocation",
                        value: "Location",
                        onclick: move |_| {
                            if !draft.is_empty() {
                                let loc = if let Ok(zip) = draft.parse::<usize>() {
                                    format!("zip={zip}")
                                } else {
                                    format!("q={draft}")
                                };
                                set_search_history.modify(|sh| {
                                    let mut v: Vec<String> = sh.iter().filter(|x| x.as_str() != loc).cloned().collect();
                                    v.push(loc.clone());
                                    set_history(&v).expect("Failed to set history");
                                    v
                                });
                                set_search_history.needs_update();
                                set_location(loc.clone());
                                set_current_loc.set(loc);
                                set_current_loc.needs_update();
                                set_draft.set(String::new());
                                set_draft.needs_update();
                            }
                        },
                    },
                },
                select {
                    id: "history-selector",
                    onchange: move |x| {
                        let s = x.data.value.as_str();
                        set_search_history.modify(|sh| {
                            let mut v: Vec<String> = sh.iter().filter(|x| x.as_str() != s).cloned().collect();
                            v.push(s.into());
                            set_history(&v).expect("Failed to set history");
                            v
                        });
                        set_search_history.needs_update();
                        set_location(s.into());
                    },
                    search_history.iter().rev().enumerate().map(|(idx, s)| {
                        let selected = s == location;
                        rsx! {
                            option {
                                key: "search-history-key-{idx}",
                                value: "{s}",
                                selected: "{selected}",
                                "{s}"
                            }
                        }
                    })
                },
                div {},
                iframe {
                    src: "{BASE_URL}/{url_path}?{location}",
                    id: "weather-frame",
                    height: "100",
                    width: "100",
                    align: "center",
                },
                script {[include_str!("../templates/scripts.js")]},
            }
        }
    })
}

fn set_history(history: &[String]) -> Result<(), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window"))?;
    let local_storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("No local storage"))?;
    let history_str = serde_json::to_string(history).map_err(|e| {
        let e: JsValue = format!("{e}").into();
        e
    })?;
    local_storage.set_item("history", &history_str)?;
    Ok(())
}

fn get_history() -> Result<Vec<String>, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window"))?;
    let local_storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("No local storage"))?;
    match local_storage.get_item("history")? {
        Some(s) => serde_json::from_str(&s).map_err(|e| {
            let e: JsValue = format!("{e}").into();
            e
        }),
        None => Ok(vec![String::from("zip=10001")]),
    }
}
