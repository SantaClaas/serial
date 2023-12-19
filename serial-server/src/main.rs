use axum::body::Body;
use axum::extract::{FromRef, Path, RawQuery, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use http::{HeaderMap, Request};
use leptos::logging::log;
use leptos::{provide_context, LeptosOptions};
use leptos_axum::{handle_server_fns_with_context, LeptosRoutes};
use serial_server::app::App;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "ssr")]
use serial_server::state::{AppState, OurState};

/// This is required for serverside state
#[cfg(feature = "ssr")]
async fn handle_server_functions(
    State(app_state): State<AppState>,
    path: Path<String>,
    headers: HeaderMap,
    raw_query: RawQuery,
    request: Request<Body>,
) -> impl IntoResponse {
    log!("Path {:?}", path);

    // We write our own handler instead of using the default leptos one to pass additional state?
    // Everything is passed on except the additionally added context
    handle_server_fns_with_context(
        path,
        headers,
        raw_query,
        move || {
            provide_context(app_state.our_state.clone());
        },
        request,
    )
    .await
}

#[cfg(feature = "ssr")]
async fn leptos_routes_handler(
    State(app_state): State<AppState>,
    request: Request<Body>,
) -> Response {
    let handler = leptos_axum::render_route_with_context(
        app_state.options.clone(),
        app_state.routes.clone(),
        move || provide_context(app_state.our_state.clone()),
        App,
    );

    handler(request).await.into_response()
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{routing::post, Router};
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use serial_server::app::*;
    use serial_server::fileserv::file_and_error_handler;

    simple_logger::init_with_level(log::Level::Info).expect("couldn't initialize logging");

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let configuration = get_configuration(None).await.unwrap();
    let leptos_options = configuration.leptos_options;
    let address = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app_state = AppState {
        our_state: Arc::new(Mutex::new(OurState::default())),
        options: leptos_options,
        routes: routes.clone(),
    };

    // Build our application with a route
    let app = Router::new()
        .route("/api/*fn_name", post(handle_server_functions))
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        // .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        // .with_state(leptos_options);
        .with_state(app_state);

    // Run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log::info!("listening on http://{}", &address);
    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
