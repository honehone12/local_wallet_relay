use std::{
    convert::Infallible,
    net::{SocketAddr, Ipv4Addr},
    path::PathBuf, 
    time::Duration, 
    sync::{Arc, Mutex}
};
use serde::{Serialize, Deserialize};
use tokio::{time, sync::RwLock};
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
    Json, Router, Server, 
    error_handling::HandleErrorLayer
};

#[derive(Serialize, Deserialize, Default, Clone)]
struct RpcPayload {
    #[serde(rename = "type")]
    type_name: String,
    function: String,
    arguments: Vec<String>,
    type_arguments: Vec<String>
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Address {
    hex: String
}

#[derive(Serialize)]
struct RpcResponse {
    message: String
}

type TempPayloadMemory = Arc<Mutex<Option<RpcPayload>>>;
type TempAddressMemory = Arc<RwLock<Option<Address>>>;

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    tracing_subscriber::fmt()
        .with_file(true).compact()
        .with_line_number(true)
        .init();

    let browser_side_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080);
    let application_side_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8081);

    let payload_memory = TempPayloadMemory::default();
    let address_memory = TempAddressMemory::default();
    
    // let assets_path = "./assets".to_string();
    let assets_path = "../../client/aptos-wallet/build".to_string();
    let assets_dir = PathBuf::from(assets_path);
    let static_files_service = ServeDir::new(assets_dir)
        .append_index_html_on_directories(true);
    let browser_side_route = Router::new()
        .fallback_service(static_files_service)
        .route("/sse", get(payload_sse_handler))
        .with_state(payload_memory.clone())
        .route("/address", post(address_fetch_handler))
        .with_state(address_memory.clone())
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
        );

    let application_side_route = Router::new()
        .route("/", get(address_request_handler))
        .with_state(address_memory.clone())
        .route("/", post(transaction_handler))
        .with_state(payload_memory.clone())
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
        );

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

async fn payload_sse_handler(State(payload_memory): State<TempPayloadMemory>)
-> Result<impl IntoResponse, StatusCode> {
    if let Ok(mut mem) = payload_memory.lock() {
        if let Some(payload) = mem.as_ref() {
            if let Ok(text) = serde_json::to_string(payload) {
                *mem = None;
                return Ok(prepare_event("payload", text, 10))
            }
        }
    }
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}

fn prepare_event(name: &'static str, text: String, interval: u64) 
-> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(move || 
        Event::default().event(name).data(text.clone())
    )
    .map(Ok)
    .throttle(Duration::from_secs(interval));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(interval))
            .text("keep-alive-text")
    )
}

async fn address_fetch_handler(State(address_memory): State<TempAddressMemory>, Json(address): Json<Address>) 
-> Result<impl IntoResponse, StatusCode> {
    let mut mem = address_memory.write().await;
    *mem = Some(address);
    let res = RpcResponse{
        message: "Ok".to_string()
    }; 
    Ok(Json(res))
}

async fn transaction_handler(State(payload_memory): State<TempPayloadMemory>, Json(payload): Json<RpcPayload>)
-> Result<impl IntoResponse, StatusCode> {
    if let Ok(()) = webbrowser::open("http://127.0.0.1:8080/?payload=true") {
        if let Ok(mut mem) = payload_memory.lock() {
            *mem = Some(payload);
            let res = RpcResponse{
                message: "Ok".to_string()
            }; 
            return Ok(Json(res))
        }
    }
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn address_request_handler(State(address_memory): State<TempAddressMemory>) 
-> Result<impl IntoResponse, StatusCode> {
    if let Ok(()) = webbrowser::open("http://127.0.0.1:8080/?address=true") {
        while address_memory.read().await.is_none() {
            time::sleep(Duration::from_millis(100)).await;
        }

        let mut mem = address_memory.write().await;
        if let Some(addr) = mem.as_ref() {
            let res = RpcResponse{
                message: addr.hex.clone()
            }; 
            *mem = None;
            return Ok(Json(res))
        }
    }
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}