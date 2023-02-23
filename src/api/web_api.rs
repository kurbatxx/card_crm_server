use axum::{
    debug_handler,
    extract::{self, Query},
    Extension,
};
use futures::executor;
//use once_cell::sync::Lazy;
use reqwest::{tls, Client};
use serde::Deserialize;
use std::{collections::HashMap, ops::Not};
use tl::NodeHandle;
use tokio::fs;

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
    dbg!(&clients);

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await {
            return "уже авторизован".to_string();
        }
    }

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
                .and_modify(|web| {
                    web.cookie = cookie_string;
                    web.password = payload.password
                });
        });

        return "login".to_string();
    };

    executor::block_on(async { clients.write().await.remove(&payload.login) });
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

async fn check_auth(web_client: &WebClient) -> bool {
    let clients_click = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "mainMenuSubView:mainMenuForm:mainMenuselectedItemName",
            "showClientListMenuItem",
        ),
        (
            "panelMenuStatemainMenuSubView:mainMenuForm:clientGroupMenu",
            "opened",
        ),
        (
            "panelMenuActionmainMenuSubView:mainMenuForm:showClientListMenuItem",
            "mainMenuSubView:mainMenuForm:showClientListMenuItem",
        ),
        (
            "mainMenuSubView:mainMenuForm",
            "mainMenuSubView:mainMenuForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "mainMenuSubView:mainMenuForm:showClientListMenuItem",
            "mainMenuSubView:mainMenuForm:showClientListMenuItem",
        ),
    ];

    let resp = web_client
        .client
        .post(SITE_URL)
        .form(&HashMap::from(clients_click))
        .send()
        .await
        .unwrap();

    let cookie = resp.cookies().next().unwrap().value().to_string();
    if cookie == web_client.cookie {
        return true;
    }
    false
}

#[derive(Deserialize)]
pub struct Name {
    login: String,
}

pub async fn get_organizations(
    Extension(clients): Extension<Clients>,
    Query(name): Query<Name>,
) -> String {
    let web_client = create_client_or_send_exist(&name.login, &clients).await;
    let cookie = &web_client.cookie.to_string();
    if cookie.is_empty() {
        executor::block_on(async { clients.write().await.remove(&name.login) });
        return "такого пользователя нет".to_string();
    }

    if check_auth(&web_client).await.not() {
        return "пока не авторизован".to_string();
    }

    let client = &web_client.client;

    let org_modal = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "workspaceSubView:workspaceForm",
            "workspaceSubView:workspaceForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_6pc51",
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_6pc51",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(org_modal))
        .send()
        .await
        .unwrap();

    fs::write("foo.html", resp.text().await.unwrap())
        .await
        .unwrap();

    let ou = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_24pc22",
            "1",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm",
            "orgSelectSubView:modalOrgSelectorForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_25pc22",
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_25pc22",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(ou))
        .send()
        .await
        .unwrap();

    fs::write("foo1.html", resp.text().await.unwrap())
        .await
        .unwrap();

    return "тут будет результат".to_string();
}
