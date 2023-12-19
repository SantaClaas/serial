use leptos::*;
use leptos::leptos_dom::logging::console_log;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SetFanSpeedError {
    /// The value was either larger than 1 or smaller than 0
    SpeedOutOfRange,
}

#[server(SetFanSpeed)]
pub async fn set_fan_speed(speed: f32) -> Result<Result<(), SetFanSpeedError>, ServerFnError> {
    if 0. > speed || speed > 1. {
        // This feels weird
        return Ok(Err(SetFanSpeedError::SpeedOutOfRange));
    }

    dbg!(speed);

    Ok(Ok(()))
}

#[component]
fn FanSpeedConfiguration() -> impl IntoView {

    let fan_speed_change = |event| {
        let value = event_target_value(&event);
        // console_log(&*format!("{}", value));

        let value = value.parse::<f32>();

        console_log(&*format!("{:?}", value));

        if let Ok(value) = value {
            spawn_local(async move {
                let _ = set_fan_speed(value).await;
            })
        }
    };

    view! {
        <input id="fan_speed" type="range" min="0" max="1" step="0.1" on:input=fan_speed_change/>
        <label for="fan_speed">Fan</label>
    }
}