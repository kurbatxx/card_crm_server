use axum::{
    debug_handler,
    extract::{self, Query},
    Extension,
};
use futures::executor;

use reqwest::{tls, Client};
use serde::{Deserialize, Serialize};
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
    pub search_query: Option<SearchQuery>,
}

// #[debug_handler]
// pub async fn nlogin(
//     Extension(clients): Extension<Clients>,
//     extract::Json(payload): extract::Json<AccessData>,
// ) -> String {
//     return "login".to_string();
// }

pub async fn logout(Extension(clients): Extension<Clients>, Query(name): Query<Name>) -> String {
    dbg!(&name.login);
    let web_client = create_client_or_send_exist(&name.login, &clients).await;
    let client = web_client.client;

    let _resp = client
        .post(AUTH_URL)
        .form(&HashMap::from([
            ("AJAXREQUEST", "j_id_jsp_659141934_0"),
            ("headerForm", "headerForm"),
            ("autoScroll", ""),
            ("javax.faces.ViewState", "j_id1"),
            (
                "headerForm:j_id_jsp_659141934_66",
                "headerForm:j_id_jsp_659141934_66",
            ),
        ]))
        .send()
        .await
        .unwrap();

    executor::block_on(async { clients.write().await.remove(&name.login) });

    "logout".to_string()
}

#[debug_handler]
pub async fn login(
    Extension(clients): Extension<Clients>,
    extract::Json(payload): extract::Json<AccessData>,
) -> String {
    let web_client = create_client_or_send_exist(&payload.login, &clients).await;

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await {
            return "?????? ??????????????????????".to_string();
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
            search_query: None,
        })
        .to_owned()
}

async fn check_auth(web_client: &WebClient) -> bool {
    let click_clients_list = [
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
        .form(&HashMap::from(click_clients_list))
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

#[derive(Debug, Serialize)]
pub struct Organizaton {
    pub id: i32,
    pub short_name: String,
    pub full_name: String,
    pub address: String,
}

pub async fn get_organizations(
    Extension(clients): Extension<Clients>,
    Query(name): Query<Name>,
) -> String {
    let web_client = create_client_or_send_exist(&name.login, &clients).await;
    let cookie = &web_client.cookie.to_string();
    if cookie.is_empty() {
        executor::block_on(async { clients.write().await.remove(&name.login) });
        return "???????????? ???????????????????????? ??????".to_string();
    }

    if check_auth(&web_client).await.not() {
        return "???????? ???? ??????????????????????".to_string();
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

    let click_ou = [
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
        .form(&HashMap::from(click_ou))
        .send()
        .await
        .unwrap();

    let mut org_html = resp.text().await.unwrap();
    if !check_first_org(&org_html) {
        org_html = click_on_org_page(1, client).await;
    }

    let mut full_org_list: Vec<Organizaton> = vec![];
    let org_page_vec = parse_org_page(&org_html);
    full_org_list.extend(org_page_vec);

    let mut org_page_num = 2;
    dbg!(org_page_num);

    loop {
        let html = click_on_org_page(org_page_num, client).await;
        let org_page_vec = parse_org_page(&html);
        full_org_list.extend(org_page_vec);
        if !check_next_org_page(&html) {
            break;
        }
        org_page_num += 1;
    }

    let rez = serde_json::to_string(&full_org_list).unwrap();

    return rez;
}

async fn click_on_org_page(number: i32, client: &Client) -> String {
    let click_org_page = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        ("orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_5pc22", ""),
        ("orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_12pc22", ""),
        ("orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_15pc22", ""),
        ("orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_18pc22", ""),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_24pc22",
            "0",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm",
            "orgSelectSubView:modalOrgSelectorForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        ("ajaxSingle", "orgSelectSubView:modalOrgSelectorForm:modalOrgSelectorOrgTable:j_id_jsp_685543358_39pc22"),
        (
            "orgSelectSubView:modalOrgSelectorForm:modalOrgSelectorOrgTable:j_id_jsp_685543358_39pc22",
            &number.to_string(),
        ),
        ("AJAX:EVENTS_COUNT", "1"),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(click_org_page))
        .send()
        .await
        .unwrap();

    dbg!(resp.status());

    resp.text().await.unwrap()
}

fn parse_org_page(org_html: &String) -> Vec<Organizaton> {
    let dom = tl::parse(&org_html, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    let element = dom
        .get_element_by_id("orgSelectSubView:modalOrgSelectorForm:modalOrgSelectorOrgTable:tb")
        .unwrap_or(NodeHandle::new(0))
        .get(parser)
        .unwrap();

    let table = &element.inner_html(parser);

    let dom = tl::parse(table, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    let org_page: Vec<Organizaton> = dom
        .query_selector("tr")
        .unwrap()
        .map(|f| {
            let row = f.get(parser).unwrap().inner_html(parser);
            let dom = tl::parse(&row, tl::ParserOptions::default()).unwrap();
            let parser = dom.parser();
            let cells: Vec<_> = dom
                .query_selector("td")
                .unwrap()
                .map(|n| n.get(parser).unwrap().inner_text(parser).to_string())
                .collect();

            let org = Organizaton {
                id: cells[0].parse::<i32>().unwrap(),
                short_name: cells[1].to_string(),
                full_name: cells[1].to_string(),
                address: cells[2].to_string(),
            };
            org
        })
        .collect();

    org_page
}

fn check_first_org(org_html: &String) -> bool {
    let button_table = button_table_dom(org_html);

    let dom = tl::parse(&button_table, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();
    let td = dom
        .query_selector(".dr-dscr-act.rich-datascr-act")
        .unwrap()
        .next()
        .unwrap()
        .get(parser)
        .unwrap();

    match td.inner_text(parser).to_string().as_str() {
        "1" => {
            println!("{}", "First");
            true
        }
        _ => {
            dbg!(td.inner_text(parser).to_string().as_str());
            false
        }
    }
}

fn check_next_org_page(org_html: &String) -> bool {
    let button_table = button_table_dom(org_html);
    let dom = tl::parse(&button_table, tl::ParserOptions::default()).unwrap();
    //let parser = dom.parser();
    let td: Vec<NodeHandle> = dom
        .query_selector(".dr-dscr-button.rich-datascr-button")
        .unwrap()
        .collect();

    if td.len() == 1 {
        return false;
    }
    true
}

fn button_table_dom(org_html: &String) -> String {
    let dom = tl::parse(&org_html, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    let element = dom
        .get_element_by_id("orgSelectSubView:modalOrgSelectorForm:modalOrgSelectorOrgTable:j_id_jsp_685543358_39pc22_table")
        .unwrap_or(NodeHandle::new(0))
        .get(parser)
        .unwrap();

    let button_table = &element.inner_html(parser);
    button_table.to_string()
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchQuery {
    pub search: String,
    pub school_id: i32,
    pub deleted: bool,
}

pub async fn init_search(
    Extension(clients): Extension<Clients>,
    Query(name): Query<Name>,
    extract::Json(payload): extract::Json<SearchQuery>,
) -> String {
    let mut web_client = create_client_or_send_exist(&name.login, &clients).await;

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await.not() {
            return "???? ??????????????????????".to_string();
        }
    }

    web_client.search_query = Some(payload.clone());
    clients
        .blocking_write()
        .insert(name.login.clone(), web_client);

    let web_client = create_client_or_send_exist(&name.login, &clients).await;
    dbg!(&web_client.search_query);

    let (id, full_name) = convert_to_id_and_fullname(payload.search.to_string());

    let client = web_client.client;
    let _ = client.get(SITE_URL).send().await.unwrap();

    let click_ou = [
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
        .form(&HashMap::from(click_ou))
        .send()
        .await
        .unwrap();

    let click_delete_filter_ou = [
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
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_6pc22",
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_6pc22",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(click_delete_filter_ou))
        .send()
        .await
        .unwrap();

    let submit_org_filter = [
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
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_43pc22",
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_43pc22",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(submit_org_filter))
        .send()
        .await
        .unwrap();

    let id_str = &id.to_string();

    let search_param = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_1pc51",
            "true",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_8pc51",
            "on",
        ),
        (
            //???????????????????? ??????????????????
            "workspaceSubView:workspaceForm:workspacePageSubView:showDeletedClients",
            if payload.deleted { "on" } else { "" }, //"on",
        ),
        (
            //ID
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_12pc51",
            if id == 0 { "" } else { id_str },
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_18pc51",
            "-1",
        ),
        (
            //??????????????
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_26pc51",
            full_name.last_name.as_str(),
        ),
        (
            //??????
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_30pc51",
            full_name.name.as_str(),
        ),
        (
            //????????????????
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_34pc51",
            full_name.surname.as_str(),
        ),
        (
            //0 ???? ?????????? ???????????? ????????
            //1 ???????? ??????????
            //2 ?????? ????????
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_43pc51",
            //&search_request.cards.to_string(),
            "0",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_46pc51",
            "0",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51",
            "j_id_jsp_635818149_109pc51",
        ),
        (
            "workspaceSubView:workspaceForm",
            "workspaceSubView:workspaceForm",
        ),
        ("javax.faces.ViewState", "j_id1"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51",
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(search_param))
        .send()
        .await
        .unwrap();

    // fs::write("search_res.html", resp.text().await.unwrap())
    //     .await
    //     .unwrap();

    let org_client_page = resp.text().await.unwrap();
    let org_clients: Vec<OrgClient> = parse_clients_page(&org_client_page);
    let rez = serde_json::to_string(&org_clients).unwrap();

    rez.to_string()
}

#[derive(Serialize)]
pub struct OrgClient {
    pub id: String,
    pub name: String,
    pub group: String,
    pub org: String,
    pub balance: String,
}

fn parse_clients_page(client_html: &String) -> Vec<OrgClient> {
    let dom = tl::parse(&client_html, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    let element = dom
        .get_element_by_id("workspaceSubView:workspaceForm:workspacePageSubView:clientListTable")
        .unwrap_or(NodeHandle::new(0))
        .get(parser)
        .unwrap();

    let table = &element.inner_html(parser);

    let mut skip_rows = 2;
    let buttons = dom.get_element_by_id("workspaceSubView:workspaceForm:workspacePageSubView:clientListTable:j_id_jsp_635818149_104pc51_table");
    dbg!(buttons);

    if buttons.is_some() {
        skip_rows += 1;
    }

    let dom = tl::parse(table, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    let org_page: Vec<OrgClient> = dom
        .query_selector("tr")
        .unwrap()
        .skip(skip_rows)
        .map(|f| {
            let row = f.get(parser).unwrap().inner_html(parser);
            let dom = tl::parse(&row, tl::ParserOptions::default()).unwrap();
            let parser = dom.parser();
            let cells: Vec<_> = dom
                .query_selector("td")
                .unwrap()
                .map(|n| n.get(parser).unwrap().inner_text(parser).to_string())
                .collect();

            let org_client = OrgClient {
                id: cells[1].to_string(),
                name: cells[3].to_string(),
                group: cells[4].to_string(),
                org: cells[6].to_string(),
                balance: cells[7].to_string(),
            };

            org_client
        })
        .collect();

    org_page
}

struct FullName {
    name: String,
    last_name: String,
    surname: String,
}

fn convert_to_id_and_fullname(search: String) -> (i32, FullName) {
    let search = search.trim();
    let id = search.parse::<i32>().unwrap_or_default();

    let mut full_name = FullName {
        name: "".to_string(),
        last_name: "".to_string(),
        surname: "".to_string(),
    };

    if id == 0 {
        let arr: Vec<_> = search.split_whitespace().collect();
        match arr.len() {
            0 => {}
            1 => full_name.last_name = arr[0].to_string(),
            2 => {
                if arr[0].contains("*").not() {
                    full_name.last_name = arr[0].to_string();
                }

                if arr[1].contains("*").not() {
                    full_name.name = arr[1].to_string();
                }
            }
            3.. => {
                if arr[0].contains("*").not() {
                    full_name.last_name = arr[0].to_string();
                }

                if arr[1].contains("*").not() {
                    full_name.name = arr[1].to_string();
                }

                if arr[2].contains("*").not() {
                    full_name.surname = arr[2].to_string();
                }
            }
            _ => {}
        }
    }

    (id, full_name)
}
