#![allow(clippy::too_many_lines)]
#![allow(clippy::used_underscore_binding)]

use anyhow::{format_err, Error};
use dioxus::{
    core::exports::futures_channel::oneshot::{channel, Sender},
    prelude::{
        dioxus_elements, fc_to_builder, format_args_f, rsx, use_future, use_state, Element,
        LazyNodes, NodeFactory, Props, Scope, VNode,
    },
};
use im_rc::HashMap;
use log::debug;
use serde::Deserialize;
use stack_string::StackString;
use time::UtcOffset;
use url::Url;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};
use http::method::Method;

use weather_util_rust::{
    latitude::Latitude, longitude::Longitude, weather_api::WeatherLocation,
    weather_data::WeatherData, weather_forecast::WeatherForecast,
};

static DEFAULT_STR: &str = "11106";
static API_ENDPOINT: &str = "https://cloud.ddboline.net/weather/";

#[derive(Copy, Clone, Default, Deserialize, Debug)]
struct Location {
    latitude: Latitude,
    longitude: Longitude,
}

#[derive(Clone, Debug)]
struct WeatherEntry {
    weather: Option<WeatherData>,
    forecast: Option<WeatherForecast>,
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    debug!("{:?}", WeatherData::default());
    dioxus::web::launch(app);
}

fn app(cx: Scope<()>) -> Element {
    let (send, recv) = channel();

    let default_cache: HashMap<WeatherLocation, WeatherEntry> = HashMap::new();
    let mut default_location_cache: HashMap<String, WeatherLocation> = HashMap::new();
    default_location_cache.insert(DEFAULT_STR.into(), get_parameters(DEFAULT_STR));

    let (cache, set_cache) = use_state(&cx, || default_cache).split();
    let (location_cache, set_location_cache) = use_state(&cx, || default_location_cache).split();
    let (location, set_location) = use_state(&cx, || get_parameters(DEFAULT_STR)).split();
    let (weather, set_weather) = use_state(&cx, WeatherData::default).split();
    let (forecast, set_forecast) = use_state(&cx, WeatherForecast::default).split();
    let (draft, set_draft) = use_state(&cx, String::new).split();
    let (search_history, set_search_history) =
        use_state(&cx, || vec![StackString::from(DEFAULT_STR)]).split();

    let location_future = use_future(&cx, (), |_| async move {
        if update_location(send).is_ok() {
            if let Ok(location) = recv.await {
                return Some(location);
            }
        }
        None
    });

    let weather_future = use_future(&cx, location, |l| {
        let entry_opt = cache.get(&l).cloned();
        async move {
            if let Some(entry) = entry_opt {
                entry
            } else {
                get_weather_data_forecast(&l).await
            }
        }
    });

    cx.render({
        if let Some(Some(location)) = location_future.value() {
            set_location.modify(|_| get_parameters(&format!("{},{}", location.latitude, location.longitude)));
            set_location.needs_update();
        }
        if let Some(entry) = weather_future.value() {
            set_cache.modify(|c| {
                let new_cache = c.update(location.clone(), entry.clone());
                if let Some(WeatherEntry{weather, forecast}) = new_cache.get(location) {
                    if let Some(weather) = weather {
                        debug!("weather_future {location:?}");
                        set_weather.modify(|_| weather.clone());
                        set_weather.needs_update();
                    }
                    if let Some(forecast) = forecast {
                        debug!("forecast_future {location:?}");
                        set_forecast.modify(|_| forecast.clone());
                        set_forecast.needs_update();
                    }
                }
                new_cache
            });
            set_cache.needs_update();
        }
        rsx!(
            link { rel: "stylesheet", href: "https://unpkg.com/tailwindcss@^2.0/dist/tailwind.min.css" },
            div { class: "mx-auto p-4 bg-gray-100 h-screen flex justify-center",
                div { class: "flex items-center justify-center flex-col",
                    div {
                        div { class: "inline-flex flex-col justify-center relative text-gray-500",
                            div { class: "relative",
                                input { class: "p-2 pl-8 rounded border border-gray-200 bg-gray-200 focus:bg-white focus:outline-none focus:ring-2 focus:ring-yellow-600 focus:border-transparent",
                                    placeholder: "search...",
                                    "type": "text",
                                    value: "{draft}",
                                    oninput: move |evt| {
                                        let msg = evt.value.as_str();
                                        set_draft.modify(|_| msg.into());
                                        set_draft.needs_update();
                                        let new_location = location_cache.get(msg).map_or_else(
                                            || {
                                                let l = get_parameters(msg);
                                                set_location_cache.modify(|lc| lc.update(msg.into(), l.clone()));
                                                l
                                            }, Clone::clone
                                        );
                                        if let Some(WeatherEntry{weather, forecast}) = cache.get(&new_location) {
                                            if let Some(weather) = weather {
                                                debug!("weather_oninput {location:?}");
                                                set_weather.modify(|_| weather.clone());
                                                set_weather.needs_update();
                                            }
                                            if let Some(forecast) = forecast {
                                                debug!("forecast_oninput {location:?}");
                                                set_forecast.modify(|_| forecast.clone());
                                                set_forecast.needs_update();
                                            }
                                            set_location.modify(|_| new_location);
                                            set_location.needs_update();
                                        }
                                    },
                                    onkeydown: move |evt| {
                                        let new_location = location_cache.get(draft).map_or_else(
                                            || {
                                                let l = get_parameters(draft);
                                                set_location_cache.modify(|lc| lc.update(draft.into(), l.clone()));
                                                l
                                            }, Clone::clone
                                        );
                                        if let Some(WeatherEntry{weather, forecast}) = cache.get(&new_location) {
                                            if let Some(weather) = weather {
                                                debug!("weather_onkeydown {location:?}");
                                                set_weather.modify(|_| weather.clone());
                                                set_weather.needs_update();
                                            }
                                            if let Some(forecast) = forecast {
                                                debug!("forecast_onkeydown {location:?}");
                                                set_forecast.modify(|_| forecast.clone());
                                                set_forecast.needs_update();
                                            }
                                        }
                                        if evt.key == "Enter" {
                                            set_draft.modify(|_| "".into());
                                            set_draft.needs_update();
                                            set_search_history.modify(|sh| {
                                                let mut v: Vec<StackString> = sh.iter().filter(|s| s.as_str() != draft.as_str()).cloned().collect();
                                                v.push(draft.into());
                                                v
                                            });
                                            set_location.modify(|_| new_location);
                                            set_location.needs_update();
                                        }
                                    },
                                }
                                svg { class: "w-4 h-4 absolute left-2.5 top-3.5",
                                    "viewBox": "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    path {
                                        d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z",
                                        "stroke-linejoin": "round",
                                        "stroke-linecap": "round",
                                        "stroke-width": "2",
                                    }
                                }
                            }
                        }
                        select { class: "bg-white border border-gray-100 w-full mt-2",
                            id: "history-selector",
                            onchange: move |x| {
                                let s = x.data.value.as_str();
                                let new_location = location_cache.get(s).map_or_else(|| {
                                    let l = get_parameters(s);
                                    set_location_cache.modify(|lc| lc.update(s.into(), l.clone()));
                                    set_location_cache.needs_update();
                                    set_search_history.modify(|sh| {
                                        let mut v: Vec<StackString> = sh.iter().filter(|x| x.as_str() != s).cloned().collect();
                                        v.push(s.into());
                                        v
                                    });
                                    set_search_history.needs_update();
                                    l
                                }, Clone::clone);
                                debug!("{new_location:?}");
                                if let Some(WeatherEntry{weather, forecast}) = cache.get(&new_location) {
                                    if let Some(weather) = weather {
                                        debug!("weather {new_location:?}");
                                        set_weather.modify(|_| weather.clone());
                                        set_weather.needs_update();
                                    }
                                    if let Some(forecast) = forecast {
                                        debug!("forecast {new_location:?}");
                                        set_forecast.modify(|_| forecast.clone());
                                        set_forecast.needs_update();
                                    }
                                }
                                set_location.modify(|_| new_location);
                                set_location.needs_update();
                            },
                            {search_history.iter().rev().map(|s| rsx! {
                                option { class: "pl-8 pr-2 py-1 border-b-2 border-gray-100 relative cursor-pointer hover:bg-yellow-50 hover:text-gray-900",
                                    key: "search-history-key-{s}",
                                    value: "{s}",
                                    "{s}"
                                }
                            })}
                        }
                    }
                    div { class: "flex flex-wrap w-full px-2",
                        div { class: "bg-gray-900 text-white relative min-w-0 break-words rounded-lg overflow-hidden shadow-sm mb-4 w-full bg-white dark:bg-gray-600",
                            div { class: "px-6 py-6 relative",
                                country_info( weather: weather, forecast: forecast )
                                country_data( weather: weather, forecast: forecast )
                            }
                            week_weather( weather: weather, forecast: forecast )
                        }
                    }
                }
            }
        )
    })
}

#[allow(clippy::used_underscore_binding)]
#[derive(Props)]
struct WeatherForecastProp<'a> {
    weather: &'a WeatherData,
    forecast: &'a WeatherForecast,
}

fn country_data<'a>(cx: Scope<'a, WeatherForecastProp<'a>>) -> Element {
    let weather = cx.props.weather;
    let temp = weather.main.temp.fahrenheit();
    let feels = weather.main.feels_like.fahrenheit();
    let min = weather.main.temp_min.fahrenheit();
    let max = weather.main.temp_max.fahrenheit();

    cx.render(rsx!(
        div { class: "block sm:flex justify-between items-center flex-wrap",
            div { class: "w-full sm:w-1/2",
                div { class: "flex mb-2 justify-between items-center",
                    span { "Temp" }
                    small { class: "px-2 inline-block", "{temp:0.2}°F" }
                }
            }
            div { class: "w-full sm:w-1/2",
                div { class: "flex mb-2 justify-between items-center",
                    span { "Feels like" }
                    small { class: "px-2 inline-block", "{feels:0.2}°F" }
                }
            }
            div { class: "w-full sm:w-1/2",
                div { class: "flex mb-2 justify-between items-center",
                    span { "Temp min" }
                    small { class: "px-2 inline-block", "{min:0.2}°F" }
                }
            }
            div { class: "w-full sm:w-1/2",
                div { class: "flex mb-2 justify-between items-center",
                    span { "Temp max" }
                    small { class: "px-2 inline-block", "{max:0.2}°F" }
                }
            }
        }
    ))
}

fn country_info<'a>(cx: Scope<'a, WeatherForecastProp<'a>>) -> Element {
    let weather = cx.props.weather;
    let name = &weather.name;
    let country = weather.sys.country.as_ref().map_or("", String::as_str);
    let mut main = String::new();
    let mut desc = String::new();
    let mut icon = String::new();
    if let Some(weather) = weather.weather.get(0) {
        main.push_str(&weather.main);
        desc.push_str(&weather.description);
        icon.push_str(&weather.icon);
    }
    let temp = weather.main.temp.fahrenheit();
    let fo: UtcOffset = weather.timezone.into();
    let date = weather.dt.to_offset(fo);

    cx.render(rsx!(
        div { class: "flex mb-4 justify-between items-center",
            div {
                h5 { class: "mb-0 font-medium text-xl",
                    "{name} {country}"
                }
                small {
                    img { class: "block w-8 h-8",
                        src: "http://openweathermap.org/img/wn/{icon}@2x.png",
                    }
                }
            }
            div { class: "text-right",
                h6 { class: "mb-0",
                    "{date}"
                }
                h3 { class: "font-bold text-4xl mb-0",
                    span {
                        "{temp:0.1}°F"
                    }
                }
            }
        }
    ))
}

fn week_weather<'a>(cx: Scope<'a, WeatherForecastProp<'a>>) -> Element {
    let forecast = cx.props.forecast;
    let high_low = forecast.get_high_low();
    cx.render(rsx!(
        div { class: "divider table mx-2 text-center bg-transparent whitespace-nowrap",
            span { class: "inline-block px-3", small { "Forecast" } }
        }
        div { class: "px-6 py-6 relative",
            div { class: "text-center justify-between items-center flex",
                style: "flex-flow: initial;",
                high_low.iter().map(|(d, (h, l, r, s, i))| {
                    let weekday = d.weekday();
                    let low = l.fahrenheit();
                    let high = h.fahrenheit();
                    let mut rain = String::new();
                    let mut snow = String::new();
                    if r.millimeters() > 0.0 {
                        rain = format!("R {:0.1}\"", r.inches());
                    }
                    if s.millimeters() > 0.0 {
                        snow = format!("S {:0.1}\"", s.inches());
                    }
                    let mut icon = String::new();
                    if let Some(i) = i.iter().next() {
                        icon.push_str(i);
                    }

                    rsx!(div {
                            key: "weather-forecast-key-{d}",
                            class: "text-center mb-0 flex items-center justify-center flex-col",
                            span { class: "block my-1",
                                "{weekday}"
                            }
                            img { class: "block w-8 h-8",
                                src: "http://openweathermap.org/img/wn/{icon}@2x.png",
                            }
                            span { class: "block my-1",
                                "{low:0.1}F°"
                            }
                            span { class: "block my-1",
                                "{high:0.1}F°"
                            }
                            span { class: "block my-1",
                                "{rain}"
                            }
                            span { class: "block my-1",
                                "{snow}"
                            }
                        }
                    )
                })
            }
        }
    ))
}

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

async fn get_weather_data_forecast(location: &WeatherLocation) -> WeatherEntry {
    debug!("{location:?}");
    let weather = get_weather_data(location).await.ok();
    let forecast = get_weather_forecast(location).await.ok();
    WeatherEntry { weather, forecast }
}

async fn get_weather_data(loc: &WeatherLocation) -> Result<WeatherData, Error> {
    let options = loc.get_options();
    run_api("weather", &options).await
}

async fn get_weather_forecast(loc: &WeatherLocation) -> Result<WeatherForecast, Error> {
    let options = loc.get_options();
    run_api("forecast", &options).await
}

async fn run_api<T: serde::de::DeserializeOwned>(
    command: &str,
    options: &[(&'static str, String)],
) -> Result<T, Error> {
    let base_url = format!("{API_ENDPOINT}{command}");
    let url = Url::parse_with_params(&base_url, options)?;
    let json = js_fetch(&url, Method::GET)
        .await
        .map_err(|e| format_err!("{:?}", e))?;
    json.into_serde().map_err(Into::into)
}

async fn js_fetch(url: &Url, method: Method) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method(method.as_str());

    let request = Request::new_with_str_and_init(url.as_str(), &opts)?;
    let window = web_sys::window().unwrap();
    let resp = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp.dyn_into().unwrap();
    JsFuture::from(resp.json()?).await
}

fn update_location(send: Sender<Location>) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let navigator = window.navigator();
    let geolocation = navigator.geolocation()?;
    debug!("geolocation {:?}", geolocation);
    let closure = Closure::once(move |js: JsValue| {
        debug!("js {:?}", js);
        if let Ok(location) = js.into_serde::<Location>() {
            debug!("location {:?}", location);
            send.send(location).unwrap();
        }
    });
    if let Some(closure) = closure.as_ref().dyn_ref() {
        geolocation.get_current_position(closure)?;
        debug!("success");
    } else {
        debug!("failure");
    }
    Ok(())
}
