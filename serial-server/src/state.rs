
#[cfg(feature = "ssr")]
use axum::extract::FromRef;
#[cfg(feature = "ssr")]
use leptos::LeptosOptions;
#[cfg(feature = "ssr")]
use leptos_router::RouteListing;
use crate::core::Device;

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Default)]
pub struct OurState {
    pub devices: Vec<Device>,
}



#[cfg(feature = "ssr")]
#[derive(Clone, Debug, FromRef)]
pub struct AppState {
    pub our_state: OurState,

    // We can only have one state type so we need to nest everything that is needed
    // The sub-states can still be conveniently accessed though
    pub options: LeptosOptions,
    pub routes: Vec<RouteListing>,
}
