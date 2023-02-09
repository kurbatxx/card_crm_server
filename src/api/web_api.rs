use axum::extract::Query;
//use once_cell::sync::Lazy;
use reqwest::{tls, Client};
use serde::Deserialize;
use std::collections::HashMap;

const SITE_URL: &str = "https://bilim.integro.kz:8181/processor/back-office/index.faces";
const AUTH_URL: &str = "https://bilim.integro.kz:8181/processor/back-office/j_security_check";

// static CLIENT_INSTANCE: Lazy<reqwest::Client> = Lazy::new(|| {
//     println!("INIT_CLIENT");
//     reqwest::Client::builder()
//         .use_native_tls()
//         .max_tls_version(tls::Version::TLS_1_1)
//         .cookie_store(true)
//         .danger_accept_invalid_certs(true)
//         .build()
//         .unwrap()
// });

#[derive(Deserialize)]
pub struct AccessData {
    pub login: String,
    pub password: String,
}

pub async fn login(Query(query): Query<AccessData>) -> String {
    let client = create_client_or_send_exist(&query.login);
    let _ = client.get(SITE_URL).send().await.unwrap();

    let resp = client
        .post(AUTH_URL)
        .form(&HashMap::from([
            ("j_username", &query.login),
            ("j_password", &query.password),
        ]))
        .send()
        .await
        .unwrap();

    let text = resp.text().await.unwrap();
    text
}

fn create_client_or_send_exist(name: &str) -> Client {
    return reqwest::Client::builder()
        .use_native_tls()
        .max_tls_version(tls::Version::TLS_1_1)
        .cookie_store(true)
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
}
