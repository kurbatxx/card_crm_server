#[path = "api/db_api.rs"]
mod db_api;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, axum::Error>>>>>;

#[tokio::main]
async fn main() {
    let users = Users::default();

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
        .route("/web_devices", get(db_api::web_devices).with_state(pool))
        .route("/ws", get(handler).with_state(users));

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
    "It works"
}

//ws
async fn handler(ws: WebSocketUpgrade, State(users): State<Users>) -> Response {
    //ws.on_upgrade(handle_socket)
    ws.on_upgrade(move |socket| handle_socket(socket, users))
}

async fn handle_socket(socket: WebSocket, users: Users) {
    let my_id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("Welcome User {}", my_id);

    let (sender, mut receiver) = socket.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    tokio::spawn(rx.forward(sender));
    users.write().await.insert(my_id, tx);

    while let Some(msg) = receiver.next().await {
        let msg = if let Ok(msg) = msg {
            dbg!(&msg);

            match msg {
                Message::Text(_) => {
                    format!("{} + text", msg.into_text().unwrap())
                }
                Message::Close(_) => {
                    disconnect(my_id, &users).await;
                    return;
                }
                _ => return,
            }
        } else {
            disconnect(my_id, &users).await;
            // client disconnected
            return;
        };

        for (&_uid, tx) in users.read().await.iter() {
            if tx.send(Ok(Message::Text(msg.to_string()))).is_err() {
                // client disconnected
                disconnect(my_id, &users).await;
                return;
            }
        }
    }
}

async fn disconnect(my_id: usize, users: &Users) {
    println!("Good bye user {}", my_id);

    users.write().await.remove(&my_id);
}
