use axum::extract;
//use once_cell::sync::Lazy;
use reqwest::{cookie, tls, Client};
use serde::Deserialize;
use std::collections::HashMap;
use tl::NodeHandle;

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

pub async fn login(extract::Json(payload): extract::Json<AccessData>) -> String {
    let client = create_client_or_send_exist(&payload.login);
    let _ = client.get(SITE_URL).send().await.unwrap();

    let resp = client
        .post(AUTH_URL)
        .form(&HashMap::from([
            ("j_username", &payload.login),
            ("j_password", &payload.password),
        ]))
        .send()
        .await
        .unwrap();

    let _cookie = resp.cookies().next().unwrap();

    let text = resp.text().await.unwrap();

    //headerForm:sysuser
    let dom = tl::parse(&text, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();
    let element = dom
        .get_element_by_id("headerForm:sysuser")
        .unwrap_or(NodeHandle::new(0))
        .get(parser)
        .unwrap();

    if &element.inner_text(parser).to_string() == &payload.login {
        return "login".to_string();
    };

    return "error".to_string();
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
