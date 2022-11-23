#![allow(clippy::too_many_lines)]

use dioxus::prelude::{
    dioxus_elements, format_args_f, rsx, use_state, Element, LazyNodes, NodeFactory, Scope, VNode,
};
use fermi::{use_read, use_set, Atom};
use url::Url;
use wasm_bindgen::JsValue;
use weather_util_rust::weather_api::WeatherLocation;
use web_sys::window;

#[cfg(debug_assertions)]
static BASE_URL: &str = "https://www.ddboline.net";

#[cfg(not(debug_assertions))]
static BASE_URL: &str = "";

static DEFAULT_LOCATION: &str = "10001";
static LOCATION: Atom<WeatherLocation> = |_| get_parameters(DEFAULT_LOCATION);

fn main() {
    // init debug tool for WebAssembly
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();

    dioxus::web::launch(app);
}

fn app(cx: Scope) -> Element {
    let (url_path, set_url_path) = use_state(&cx, || "weather/plot.html").split();
    let (draft, set_draft) = use_state(&cx, String::new).split();
    let (current_loc, set_current_loc) = use_state(&cx, || None).split();
    let (search_history, set_search_history) = use_state(&cx, || {
        get_history().unwrap_or_else(|_| vec![String::from("zip=10001")])
    })
    .split();

    let location = use_read(&cx, LOCATION);
    let set_location = use_set(&cx, LOCATION);

    let window = window().unwrap();
    let search = window.location().search().unwrap();

    if !search.is_empty() && current_loc.is_none() {
        let s = search.trim_start_matches("?location=").to_string();
        let loc = get_parameters(&s);
        set_current_loc.set(Some(s.clone()));
        set_current_loc.needs_update();
        if !search_history.contains(&s) {
            set_search_history.modify(|sh| {
                let mut v: Vec<String> = sh.iter().filter(|x| x.as_str() != s).cloned().collect();
                v.push(s);
                set_history(&v).expect("Failed to set history");
                v
            });
            set_search_history.needs_update();
            set_location(loc);
        }
    }

    // let location_future = use_future(&cx, (), |_| async move {
    //     if let Ok(ip) = get_ip_address().await {
    //         debug!("ip {ip}");
    //         if let Ok(location) = get_location_from_ip(ip).await {
    //             debug!("location {location:?}");
    //             return Some(location);
    //         }
    //     }
    //     None
    // });

    cx.render({
        let url: Url = format!("{BASE_URL}/{url_path}").parse().expect("Failed to parse base url");
        let url = Url::parse_with_params(url.as_str(), location.get_options()).unwrap_or(url);
        // if let Some(Some(loc)) = location_future.value() {
        //     if loc != location {
        //         set_location(location.clone());
        //     }
        // }
        rsx! {
            body {
                div {
                    input {
                        "type": "button",
                        name: "update_location",
                        value: "Update Location",
                        // onclick: move |_| {
                        //     location_future.restart();
                        // },
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
                                    let loc = get_parameters(draft);
                                    if !search_history.contains(draft) {
                                        set_search_history.modify(|sh| {
                                            let mut v: Vec<String> = sh.iter().filter(|x| x.as_str() != draft).cloned().collect();
                                            v.push(draft.into());
                                            set_history(&v).expect("Failed to set history");
                                            v
                                        });
                                        set_search_history.needs_update();
                                    }
                                    set_location(loc);
                                    set_current_loc.set(Some(draft.clone()));
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
                            let s = x.data.value.as_str().to_string();
                            let loc = get_parameters(&s);
                            if !search_history.contains(&s) {
                                set_search_history.modify(|sh| {
                                    let mut v: Vec<String> = sh.iter().filter(|x| x.as_str() != s).cloned().collect();
                                    v.push(s);
                                    set_history(&v).expect("Failed to set history");
                                    v
                                });
                                set_search_history.needs_update();
                            }
                            set_location(loc);
                        },
                        search_history.iter().rev().enumerate().map(|(idx, s)| {
                            rsx! {
                                option {
                                    key: "search-history-key-{idx}",
                                    value: "{s}",
                                    "{s}"
                                }
                            }
                        })
                    },
                    input {
                        "type": "button",
                        name: "clear",
                        value: "Clear",
                        onclick: move |_| {
                            let history = vec![String::from("10001")];
                            set_history(&history).unwrap();
                            set_search_history.set(history);
                            set_search_history.needs_update();
                        }
                    },
                    div {},
                    iframe {
                        src: "{url}",
                        id: "weather-frame",
                        height: "100",
                        width: "100",
                        align: "center",
                    },
                    script {[include_str!("../templates/scripts.js")]},
                }
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

// async fn js_fetch(url: &Url, method: Method) -> Result<JsValue, JsValue> {
//     let mut opts = RequestInit::new();
//     opts.method(method.as_str());

//     let request = Request::new_with_str_and_init(url.as_str(), &opts)?;
//     let window = web_sys::window().unwrap();
//     let resp = JsFuture::from(window.fetch_with_request(&request)).await?;
//     let resp: Response = resp.dyn_into().unwrap();
//     JsFuture::from(resp.json()?).await
// }

// async fn text_fetch(url: &Url, method: Method) -> Result<JsValue, JsValue> {
//     let mut opts = RequestInit::new();
//     opts.method(method.as_str());

//     let request = Request::new_with_str_and_init(url.as_str(), &opts)?;
//     let window = web_sys::window().unwrap();
//     let resp = JsFuture::from(window.fetch_with_request(&request)).await?;
//     let resp: Response = resp.dyn_into().unwrap();
//     JsFuture::from(resp.text()?).await
// }

// async fn get_ip_address() -> Result<Ipv4Addr, JsValue> {
//     let url: Url = "https://ipinfo.io/ip".parse().map_err(|e| {
//         error!("error {e}");
//         let e: JsValue = format!("{e}").into();
//         e
//     })?;
//     let resp = text_fetch(&url, Method::GET).await?;
//     let resp = resp
//         .as_string()
//         .ok_or_else(|| JsValue::from_str("Failed to get ip"))?
//         .trim()
//         .to_string();
//     debug!("got resp {resp}");
//     resp.parse().map_err(|e| {
//         let e: JsValue = format!("{e}").into();
//         e
//     })
// }

// async fn get_location_from_ip(ip: Ipv4Addr) -> Result<WeatherLocation,
// JsValue> {     #[derive(Default, Serialize, Deserialize)]
//     struct Location {
//         latitude: Latitude,
//         longitude: Longitude,
//     }

//     let ipaddr = ip.to_string();
//     let url = Url::parse("https://ipwhois.app/json/")
//         .map_err(|e| {
//             error!("error {e}");
//             let e: JsValue = format!("{e}").into();
//             e
//         })?
//         .join(&ipaddr)
//         .map_err(|e| {
//             error!("error {e}");
//             let e: JsValue = format!("{e}").into();
//             e
//         })?;
//     let json = js_fetch(&url, Method::GET).await?;
//     let location: Location = serde_wasm_bindgen::from_value(json)?;
//     Ok(WeatherLocation::from_lat_lon(
//         location.latitude,
//         location.longitude,
//     ))
// }

fn get_parameters(search_str: &str) -> WeatherLocation {
    let mut opts = WeatherLocation::from_city_name(search_str);
    if let Ok(zip) = search_str.parse::<u64>() {
        opts = WeatherLocation::from_zipcode(zip);
    } else if search_str.contains(',') {
        let mut iter = search_str.split(',');
        if let Some(lat) = iter.next() {
            if let Ok(lat) = lat.parse() {
                if let Some(lon) = iter.next() {
                    if let Ok(lon) = lon.parse() {
                        opts = WeatherLocation::from_lat_lon(lat, lon);
                    }
                }
            }
        }
    }
    opts
}
