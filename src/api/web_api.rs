use axum::{debug_handler, extract, Extension};
use futures::executor;
//use once_cell::sync::Lazy;
use reqwest::{tls, Client};
use serde::Deserialize;
use std::collections::HashMap;
use tl::NodeHandle;

use crate::Clients;

const SITE_URL: &str = "https://bilim.integro.kz:8181/processor/back-office/index.faces";
const AUTH_URL: &str = "https://bilim.integro.kz:8181/processor/back-office/j_security_check";

#[derive(Deserialize)]
pub struct AccessData {
    pub login: String,
    pub password: String,
}
#[derive(Debug, Clone)]
pub struct WebClient {
    pub client: Client,
    pub cookie: String,
    pub password: String,
}

// #[debug_handler]
// pub async fn nlogin(
//     Extension(clients): Extension<Clients>,
//     extract::Json(payload): extract::Json<AccessData>,
// ) -> String {
//     return "login".to_string();
// }

#[debug_handler]
pub async fn login(
    Extension(clients): Extension<Clients>,
    extract::Json(payload): extract::Json<AccessData>,
) -> String {
    let web_client = create_client_or_send_exist(&payload.login, &clients).await;
    dbg!(&web_client);

    let client = web_client.client;
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

    let cookie = resp.cookies().next().unwrap();
    let cookie_string = cookie.value().to_string();

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
        executor::block_on(async {
            clients
                .write()
                .await
                .entry(payload.login)
                .and_modify(|web| web.cookie = cookie_string);
        });

        return "login".to_string();
    };

    return "error".to_string();
}

async fn create_client_or_send_exist(name: &str, clients: &Clients) -> WebClient {
    clients
        .write()
        .await
        .entry(name.to_string())
        .or_insert(WebClient {
            client: reqwest::Client::builder()
                .use_native_tls()
                .max_tls_version(tls::Version::TLS_1_1)
                .cookie_store(true)
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
            cookie: "".to_string(),
            password: "".to_string(),
        })
        .to_owned()
}
