use anyhow::{anyhow, Context, Result};
use iocraft::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct LocationData {
    lat: f64,
    lon: f64,
    country: String,
    region: String,
    city: String,
}

impl LocationData {
    async fn fetch() -> Result<Self> {
        Ok(surf::get("http://ip-api.com/json")
            .recv_json()
            .await
            .map_err(|e| anyhow!(e))
            .context("failed to fetch location data")?)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct WeatherDataCurrent {
    temperature_2m: f32,
    relative_humidity_2m: f32,
    precipitation_probability: f32,
    weather_code: i32,
}

impl WeatherDataCurrent {
    pub fn description(&self) -> &'static str {
        match self.weather_code {
            0 => "Clear",
            1 => "Mainly Clear",
            2 => "Partly Cloudy",
            3 => "Overcast",
            45 => "Fog",
            48 => "Depositing Rime Fog",
            51 => "Light Drizzle",
            53 => "Moderate Drizzle",
            55 => "Dense Drizzle",
            56 => "Light Freezing Drizzle",
            57 => "Dense Freezing Drizzle",
            61 => "Light Rain",
            63 => "Moderate Rain",
            65 => "Heavy Rain",
            66 => "Light Freezing Rain",
            67 => "Heavy Freezing Rain",
            71 => "Light Snow",
            73 => "Moderate Snow",
            75 => "Heavy Snow",
            77 => "Flurries",
            80 => "Slight Rain Showers",
            81 => "Moderate Rain Showers",
            82 => "Violent Rain Showers",
            85 => "Slight Snow Showers",
            86 => "Heavy Snow Showers",
            95 => "Thunderstorm",
            96 => "Thunderstorm With Slight Hail",
            97 => "Thunderstorm With Heavy Hail",
            _ => "Unknown",
        }
    }

    pub fn color(&self) -> Color {
        match self.weather_code {
            0 | 1 => Color::Yellow,
            2 | 3 => Color::Grey,
            45 | 48 => Color::White,
            56 | 57 | 66 | 67 => Color::Blue,
            51 | 53 | 55 | 61 | 63 | 65 | 81 | 82 => Color::Cyan,
            71 | 77 => Color::White,
            73 | 75 | 85 | 86 => Color::White,
            80 => Color::Cyan,
            95 | 96 | 97 => Color::Yellow,
            _ => Color::White,
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self.weather_code {
            0 | 1 => "☀️",
            2 => "⛅",
            3 => "☁️",
            45 | 48 => "🌫️",
            56 | 57 | 66 | 67 => "🌧️ 🥶",
            51 | 53 | 55 | 61 | 63 | 65 | 81 | 82 => "🌧️",
            71 | 77 => "❄️",
            73 | 75 | 85 | 86 => "🌨️",
            80 => "🌦️",
            95 | 96 | 97 => "⛈️",
            _ => "❓",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct WeatherDataCurrentUnits {
    temperature_2m: String,
    relative_humidity_2m: String,
    precipitation_probability: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct WeatherData {
    location: LocationData,
    current_units: WeatherDataCurrentUnits,
    current: WeatherDataCurrent,
}

impl WeatherData {
    async fn fetch() -> Result<Self> {
        let location = LocationData::fetch().await?;
        let mut ret: Self = surf::get(&format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,precipitation_probability",
            location.lat, location.lon
        )).recv_json().await.map_err(|e| anyhow!(e)).context("failed to fetch weather data")?;
        ret.location = location;
        Ok(ret)
    }
}

#[component]
fn LoadingIndicator(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut frame = hooks.use_state(|| 0);
    hooks.use_future(async move {
        loop {
            smol::Timer::after(std::time::Duration::from_millis(100)).await;
            frame.set((frame + 1) % FRAMES.len());
        }
    });
    element! {
        View(
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            width: 100pct,
            height: 100pct,
        ) {
            Text(content: FRAMES[frame.get()], color: Color::Yellow)
            Text(content: " Loading...")
        }
    }
}

#[derive(Default, Props)]
struct WeatherDataViewProps {
    data: WeatherData,
}

#[component]
fn WeatherDataView(props: &WeatherDataViewProps) -> impl Into<AnyElement<'static>> {
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
        ) {
            View(
                flex_direction: FlexDirection::Column,
                border_style: BorderStyle::Single,
                border_color: Color::DarkGrey,
                border_edges: Edges::Bottom,
                align_items: AlignItems::Center,
                width: 100pct,
            ) {
                View {
                    Text(content: "Weather for ")
                    Text(content: format!("{}, {}, {}", props.data.location.city, props.data.location.region, props.data.location.country))
                }
                Text(content: format!("{}º, {}º", props.data.location.lat, props.data.location.lon), color: Color::DarkGrey)
            }
            View(
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
            ) {
                View(padding: 1) {
                    Text(content: format!("{} {} {}", props.data.current.emoji(), props.data.current.description(), props.data.current.emoji()), color: props.data.current.color())
                }
                View {
                    Text(content: "Temperature: ", weight: Weight::Bold)
                    Text(content: format!("{:.1}{}", props.data.current.temperature_2m, props.data.current_units.temperature_2m))
                }
                View {
                    Text(content: "Humidity: ", weight: Weight::Bold)
                    Text(content: format!("{:.1}{}", props.data.current.relative_humidity_2m, props.data.current_units.relative_humidity_2m))
                }
                View {
                    Text(content: "Chance of Precipitation: ", weight: Weight::Bold)
                    Text(content: format!("{:.1}{}", props.data.current.precipitation_probability, props.data.current_units.precipitation_probability))
                }
            }
        }
    }
}

enum WeatherState {
    Init,
    Loading,
    Loaded(Result<WeatherData>),
}

#[component]
fn Weather(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut state = hooks.use_state(|| WeatherState::Init);
    let mut should_exit = hooks.use_state(|| false);

    let mut load = hooks.use_async_handler(move |_: ()| async move {
        state.set(WeatherState::Loading);
        state.set(WeatherState::Loaded(WeatherData::fetch().await));
    });
    if matches!(*state.read(), WeatherState::Init) {
        load(());
    }

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('q') => should_exit.set(true),
                    KeyCode::Char('r') => load(()),
                    _ => {}
                }
            }
            _ => {}
        }
    });

    if should_exit.get() {
        system.exit();
    }

    element! {
        View(
            width: 70,
            height: 14,
            margin: 1,
            border_style: BorderStyle::Round,
            border_color: Color::Cyan,
            flex_direction: FlexDirection::Column,
        ) {
            View(
                flex_grow: 1.0,
            ) {
                #(match &*state.read() {
                    WeatherState::Loaded(Ok(data)) => element! {
                        WeatherDataView(data: data.clone())
                    }.into_any(),
                    WeatherState::Loaded(Err(err)) => element! {
                        View(
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            width: 100pct,
                            height: 100pct,
                            padding: 2,
                        ) {
                            Text(content: "Error!", weight: Weight::Bold, color: Color::Red)
                            Text(content: format!("{:#}", err))
                        }
                    }.into_any(),
                    _ => element!(LoadingIndicator).into_any(),
                })
            }
            View(
                width: 100pct,
                border_style: BorderStyle::Single,
                border_color: Color::DarkGrey,
                border_edges: Edges::Top,
                padding_left: 1,
            ) {
                Text(content: "[R] Reload · [Q] Quit")
            }
        }
    }
}

fn main() {
    smol::block_on(element!(Weather).render_loop()).unwrap();
}
