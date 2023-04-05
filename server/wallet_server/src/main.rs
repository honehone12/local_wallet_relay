use std::{
    convert::Infallible,
    net::{SocketAddr, Ipv4Addr},
    path::PathBuf, 
    time::Duration, 
    sync::{Arc, RwLock},
    env
};
use serde::{Serialize, Deserialize};
use tokio_stream::StreamExt as _;
use futures::{
    stream::{self, Stream}
};
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    services::ServeDir,
    trace::TraceLayer
};
use axum::{
    routing::{get, post},
    http::StatusCode,
    response::{sse::{Sse, Event}, IntoResponse}, 
    extract::State,
    Json, Router, Server, error_handling::HandleErrorLayer 
};

#[derive(Serialize, Deserialize, Default, Clone)]
struct RpcPayload {
    #[serde(rename = "type")]
    type_name: String,
    function: String,
    arguments: Vec<String>,
    type_arguments: Vec<String>
}

#[derive(Serialize)]
struct RpcResponse {
    message: String
}

type TempMemory = Arc<RwLock<RpcPayload>>;

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    // default display
    tracing_subscriber::fmt::init();

    let browser_side_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080);
    let application_side_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8081);

    let memory = TempMemory::default();

    // let assets_path = "./assets".to_string();
    let assets_path = env!("DEV_SERVER_DIR").to_string();
    let assets_dir = PathBuf::from(assets_path);
    let static_files_service = ServeDir::new(assets_dir)
        .append_index_html_on_directories(true);
    
    let browser_side_route = Router::new()
        .fallback_service(static_files_service)
        .route("/sse", get(browser_handler))
        .with_state(memory.clone());
    let application_side_route = Router::new()
        .route("/", post(application_handler))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    if e.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", e),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner()
        )
        .with_state(memory.clone());

    tracing::info!("listening on {} for browser.", browser_side_addr);
    tracing::info!("listening on {} for application.", application_side_addr);

    let browser_server = Server::bind(&browser_side_addr)
        .serve(browser_side_route.into_make_service());
    let game_server = Server::bind(&application_side_addr)
        .serve(application_side_route.into_make_service());
    
    let result = tokio::join!(browser_server, game_server);
    match result.0 {
        Ok(_) => result.1,
        Err(_) => result.0
    }
}

async fn browser_handler(State(memory): State<TempMemory>)
-> Sse<impl Stream<Item = Result<Event, Infallible>>>  {
    let mem = memory.read().unwrap().to_owned();
    let request = serde_json::to_string(&mem).unwrap();
    *memory.write().unwrap() = RpcPayload::default();
    
    let stream = stream::repeat_with(move || Event::default().event("data").data(request.clone()))
    .map(Ok)
    .throttle(Duration::from_secs(5));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive-text")
    )
}

async fn application_handler(State(memory): State<TempMemory>, Json(payload): Json<RpcPayload>)
-> Result<impl IntoResponse, StatusCode> {
    *memory.write().unwrap() = payload;

    webbrowser::open("http://127.0.0.1:8080").unwrap();
    let res = RpcResponse{
        message: "Ok".to_string()
    }; 
    Ok(Json(res))
}