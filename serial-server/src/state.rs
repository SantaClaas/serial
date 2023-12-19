use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use std::sync::Arc;
        use axum::extract::FromRef;
        use leptos::LeptosOptions;
        use leptos_router::RouteListing;
        use tokio::sync::Mutex;
        use crate::core::Device;

        #[derive(Debug, Clone, Default)]
        pub struct OurState {
            pub devices: Vec<Device>,
        }

        #[derive(Clone, Debug, FromRef)]
        pub struct AppState {
            pub our_state: Arc<Mutex<OurState>>,

            // We can only have one state type so we need to nest everything that is needed
            // The sub-states can still be conveniently accessed though
            pub options: LeptosOptions,
            pub routes: Vec<RouteListing>,
        }
    }
}