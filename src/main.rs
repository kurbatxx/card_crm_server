use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Deserialize, Serialize)]
struct WebDevice {
    web_device_id: i32,
    web_device_name: String,
    visible: bool,
    colored: bool,
    icon: Option<String>,
}

#[tokio::main]
async fn main() {
    let db_url = dotenvy::var("DATABASE_URL").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Не удалось создать пул соединений");

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/web_devices", get(web_devices).with_state(pool))
        .route("/ws", get(handler));

    // `POST /users` goes to `create_user`
    //.route("/users", post(create_user));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3333));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn web_devices(State(pool): State<Pool<Postgres>>) -> String {
    let web_devices = sqlx::query_as!(
        WebDevice,
        r#"SELECT web_device_id, web_device_name, visible, colored, icon FROM web_device"#
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    dbg!(&web_devices);

    serde_json::to_string(&web_devices)
        .unwrap_or_else(|_| json!({"error":"Не удалось получть данные"}).to_string())
}

//ws
async fn handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

async fn handle_socket(mut socket: WebSocket) {
    let my_id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("Welcome User {}", my_id);

    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            format!("{} + text", msg.into_text().unwrap())
        } else {
            // client disconnected
            return;
        };

        if socket.send(Message::Text(msg)).await.is_err() {
            // client disconnected
            return;
        }
    }
}
