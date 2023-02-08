use axum::extract::State;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Pool, Postgres};

#[derive(Debug, Deserialize, Serialize)]
struct WebDevice {
    web_device_id: i32,
    web_device_name: String,
    visible: bool,
    colored: bool,
    icon: Option<String>,
}

pub async fn web_devices(State(pool): State<Pool<Postgres>>) -> String {
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
