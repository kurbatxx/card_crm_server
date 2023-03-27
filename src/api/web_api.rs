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

#[derive(Serialize)]
pub struct OrgClientsWithNextPage {
    pub org_clients: Vec<OrgClient>,
    pub next_page_exist: bool,
}

pub async fn logout(Extension(clients): Extension<Clients>, Query(name): Query<Name>) -> String {
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

#[derive(Deserialize, Debug)]
pub struct CurrentSearchPage {
    login: String,
    page_number: String,
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

    loop {
        let html = click_on_org_page(org_page_num, client).await;
        let org_page_vec = parse_org_page(&html);
        full_org_list.extend(org_page_vec);
        if !check_next_page(&html) {
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

// fn check_next_org_page(org_html: &String) -> bool {
//     let button_table = button_table_dom(org_html);
//     let dom = tl::parse(&button_table, tl::ParserOptions::default()).unwrap();
//     //let parser = dom.parser();
//     let td: Vec<NodeHandle> = dom
//         .query_selector(".dr-dscr-button.rich-datascr-button")
//         .unwrap()
//         .collect();

//     if td.len() == 1 {
//         return false;
//     }
//     true
// }

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
    Query(query): Query<Name>,
    extract::Json(payload): extract::Json<SearchQuery>,
) -> String {
    dbg!("init_search");
    let mut web_client = create_client_or_send_exist(&query.login, &clients).await;

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await.not() {
            return "Не авторизован".to_string();
        }
    }

    web_client.search_query = Some(payload.clone());
    clients
        .write()
        .await
        .insert(query.login.clone(), web_client);

    let web_client = create_client_or_send_exist(&query.login, &clients).await;
    dbg!(&web_client.search_query);

    let (id, full_name) = convert_to_id_and_fullname(payload.search.to_string());

    let client = web_client.client;
    let _ = client.get(SITE_URL).send().await.unwrap();

    let clear_button = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51",
            "j_id_jsp_635818149_109pc51",
        ),
        (
            "workspaceSubView:workspaceForm",
            "workspaceSubView:workspaceForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_54pc51",
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_54pc51",
        ),
        (
            "ajaxSingle",
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_54pc51",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(clear_button))
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
            //Показывать удалённых
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
            //Фамилия
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_26pc51",
            full_name.last_name.as_str(),
        ),
        (
            //Имя
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_30pc51",
            full_name.name.as_str(),
        ),
        (
            //Отчество
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_34pc51",
            full_name.surname.as_str(),
        ),
        (
            //0 не важно наличе карт
            //1 есть карты
            //2 нет карт
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
    let (org_clients, next_page_exist): (Vec<OrgClient>, bool) =
        parse_clients_page(&org_client_page);

    let org_clienst_with_next_page = OrgClientsWithNextPage {
        org_clients,
        next_page_exist,
    };

    let res = serde_json::to_string(&org_clienst_with_next_page).unwrap();

    res.to_string()
}

#[derive(Serialize)]
pub struct OrgClient {
    pub id: String,
    pub name: String,
    pub group: String,
    pub org: String,
    pub balance: String,
}

fn parse_clients_page(client_html: &String) -> (Vec<OrgClient>, bool) {
    let mut next_page_exist = false;

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

    if buttons.is_some() {
        skip_rows += 1;
        next_page_exist = check_next_page(
            &buttons
                .unwrap()
                .get(parser)
                .unwrap()
                .inner_html(parser)
                .to_string(),
        );
    }

    let dom = tl::parse(table, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    let org_clients_page: Vec<OrgClient> = dom
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

    (org_clients_page, next_page_exist)
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

pub async fn set_search_page(
    Extension(clients): Extension<Clients>,
    Query(query): Query<CurrentSearchPage>,
) -> String {
    let web_client = create_client_or_send_exist(&query.login, &clients).await;

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await.not() {
            return "Не авторизован".to_string();
        }
    }

    let client = web_client.client;

    let click_bottom_number = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51",
            "j_id_jsp_635818149_109pc51",
        ),
        (
            "workspaceSubView:workspaceForm",
            "workspaceSubView:workspaceForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:clientListTable:j_id_jsp_635818149_104pc51",
            &query.page_number,
        ),
        (
            "ajaxSingle",
            "workspaceSubView:workspaceForm:workspacePageSubView:clientListTable:j_id_jsp_635818149_104pc51",
        ),
        ("AJAX:EVENTS_COUNT", "1"),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(click_bottom_number))
        .send()
        .await
        .unwrap();

    let org_client_page = resp.text().await.unwrap();
    let (org_clients, next_page_exist): (Vec<OrgClient>, bool) =
        parse_clients_page(&org_client_page);

    let clients_with_next_page = OrgClientsWithNextPage {
        org_clients,
        next_page_exist,
    };

    let res = serde_json::to_string(&clients_with_next_page).unwrap();

    res.to_string()
}

fn check_next_page(html: &String) -> bool {
    let mut next_page_exist = false;

    let button_table = button_table_dom(html);
    let dom = tl::parse(&button_table, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();

    if let Some(i) = dom.query_selector("img[src]") {
        let images: Vec<NodeHandle> = i.collect();
        dbg!(&images);

        for img in images {
            let out_html = img.get(parser).unwrap().outer_html(parser).to_string();
            if out_html.contains("right-arrow.png") {
                next_page_exist = true;
                break;
            }
        }
    }

    dbg!(next_page_exist);
    next_page_exist
}

pub async fn download_all(
    Extension(clients): Extension<Clients>,
    Query(query): Query<Name>,
    extract::Json(payload): extract::Json<SearchQuery>,
) -> String {
    dbg!("all");
    let mut web_client = create_client_or_send_exist(&query.login, &clients).await;

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await.not() {
            return "Не авторизован".to_string();
        }
    }

    web_client.search_query = Some(payload.clone());
    clients
        .write()
        .await
        .insert(query.login.clone(), web_client);

    let web_client = create_client_or_send_exist(&query.login, &clients).await;
    dbg!(&web_client.search_query);

    let (id, full_name) = convert_to_id_and_fullname(payload.search.to_string());

    let client = web_client.client;
    let _ = client.get(SITE_URL).send().await.unwrap();

    // AJAXREQUEST	"j_id_jsp_659141934_0"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_1pc51	"true"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_5pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_8pc51	"on"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_12pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_14pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_16pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_18pc51	"-1"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_26pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_28pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_30pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_32pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_34pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_36pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_38pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_40pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_43pc51	"0"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_46pc51	"0"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_49pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_51pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_107pc51	""
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51	"j_id_jsp_635818149_109pc51"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_112pc51	"0,00"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_126pc51	"0,00"
    // workspaceSubView:workspaceForm:workspacePageSubView:removedClientDeletePanelOpenedState	""
    // workspaceSubView:workspaceForm	"workspaceSubView:workspaceForm"
    // autoScroll	""
    // javax.faces.ViewState	"j_id1"
    // workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51	"workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51"

    let search_submit = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_1pc51",
            "true",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_5pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_8pc51",
            "on",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_12pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_14pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_16pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_18pc51",
            "-1",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_26pc51",
            "", //fam
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_28pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_30pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_32pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_34pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_38pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_40pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_43pc51",
            "0",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_46pc51",
            "0",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_49pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_51pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_51pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51",
            "j_id_jsp_635818149_109pc51",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_112pc51",
            "0,00",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_126pc51",
            "0,00",
        ),

        ("workspaceSubView:workspaceForm:workspacePageSubView:removedClientDeletePanelOpenedState", 
            "",
        ),
        ("workspaceSubView:workspaceForm", "workspaceSubView:workspaceForm"),
        ("autoScroll", ""),

        ("javax.faces.ViewState", "j_id1"),
        ("workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51", "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51")

    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(search_submit))
        .send()
        .await
        .unwrap();

    fs::write("0_search_submit.html", resp.text().await.unwrap())
        .await
        .unwrap();

    let clear_button = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51",
            "j_id_jsp_635818149_109pc51",
        ),
        (
            "workspaceSubView:workspaceForm",
            "workspaceSubView:workspaceForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_54pc51",
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_54pc51",
        ),
        (
            "ajaxSingle",
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_54pc51",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(clear_button))
        .send()
        .await
        .unwrap();

    fs::write("1_clear_button.html", resp.text().await.unwrap())
        .await
        .unwrap();

    let open_org_selector = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_1pc51",
            "true",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_5pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_8pc51",
            "on",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_12pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_14pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_16pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_18pc51",
            "-1",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_26pc51",
            "", //fam
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_28pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_30pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_32pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_34pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_38pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_40pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_43pc51",
            "0",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_46pc51",
            "0",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_49pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_51pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_51pc51",
            "",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51",
            "j_id_jsp_635818149_109pc51",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_112pc51",
            "0,00",
        ),
        (
            "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_126pc51",
            "0,00",
        ),

        ("workspaceSubView:workspaceForm:workspacePageSubView:removedClientDeletePanelOpenedState", 
            "",
        ),
        ("workspaceSubView:workspaceForm", "workspaceSubView:workspaceForm"),
        ("autoScroll", ""),

        ("javax.faces.ViewState", "j_id1"),
        ("workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_6pc51", "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_6pc51")
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(open_org_selector))
        .send()
        .await
        .unwrap();

    fs::write("2_open_org_selector.html", resp.text().await.unwrap())
        .await
        .unwrap();

    // AJAXREQUEST	"j_id_jsp_659141934_0"
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_5pc22	""
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_12pc22	""
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_15pc22	""
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_18pc22	"858"
    // orgSelectSubView:modalOrgSelectorForm:regionsList	""
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_24pc22	"0"
    // orgSelectSubView:modalOrgSelectorForm	"orgSelectSubView:modalOrgSelectorForm"
    // autoScroll	""
    // javax.faces.ViewState	"j_id2"
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_19pc22	"orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_19pc22"
    // AJAX:EVENTS_COUNT	"5"

    let set_filter = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_5pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_12pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_15pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_18pc22",
            "879",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_24pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm",
            "orgSelectSubView:modalOrgSelectorForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_19pc22",
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_19pc22",
        ),
        ("AJAX:EVENTS_COUNT", "1"),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(set_filter))
        .send()
        .await
        .unwrap();

    fs::write("3_set_filter.html", resp.text().await.unwrap())
        .await
        .unwrap();

    let click_table_element = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_5pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_12pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_15pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_18pc22",
            "879",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_24pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm",
            "orgSelectSubView:modalOrgSelectorForm",
        ),
        ("autoScroll", ""),
        ("javax.faces.ViewState", "j_id1"),
        (
            "orgSelectSubView:modalOrgSelectorForm:modalOrgSelectorOrgTable:0:j_id_jsp_685543358_32pc22",
            "orgSelectSubView:modalOrgSelectorForm:modalOrgSelectorOrgTable:0:j_id_jsp_685543358_32pc22",
        ),
    ];

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(click_table_element))
        .send()
        .await
        .unwrap();

    fs::write("4_click_table_element.html", resp.text().await.unwrap())
        .await
        .unwrap();

    // AJAXREQUEST	"j_id_jsp_659141934_0"
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_5pc22	"Демо школа"
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_12pc22	""
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_15pc22	""
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_18pc22	"858"
    // orgSelectSubView:modalOrgSelectorForm:regionsList	""
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_24pc22	"0"
    // orgSelectSubView:modalOrgSelectorForm	"orgSelectSubView:modalOrgSelectorForm"
    // autoScroll	""
    // javax.faces.ViewState	"j_id2"
    // orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_43pc22	"orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_43pc22"

    let submit_selected_row = [
        ("AJAXREQUEST", "j_id_jsp_659141934_0"),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_5pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_12pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_15pc22",
            "",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_18pc22",
            "879",
        ),
        (
            "orgSelectSubView:modalOrgSelectorForm:j_id_jsp_685543358_24pc22",
            "",
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
        .form(&HashMap::from(submit_selected_row))
        .send()
        .await
        .unwrap();

    fs::write("5_submit_selected_row.html", resp.text().await.unwrap())
        .await
        .unwrap();

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(search_submit))
        .send()
        .await
        .unwrap();

    let mut res = resp.text().await.unwrap();

    fs::write("6_search_submit.html", &res).await.unwrap();

    //Первая страница с фильтром..

    let mut clienst_list: Vec<OrgClient> = vec![];

    loop {
        let (part_clients, next_page_exist) = parse_clients_page(&res);
        clienst_list.extend(part_clients);
        if next_page_exist.not() {
            break;
        }

        let next = [
            ("AJAXREQUEST", "j_id_jsp_659141934_0"),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_1pc51",
                "true",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_5pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_8pc51",
                "on",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_12pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_14pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_16pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_18pc51",
                "-1",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_26pc51",
                "", //fam
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_28pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_30pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_32pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_34pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_38pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_40pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_43pc51",
                "0",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_46pc51",
                "0",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_49pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_51pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_51pc51",
                "",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_108pc51",
                "j_id_jsp_635818149_109pc51",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_112pc51",
                "0,00",
            ),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_126pc51",
                "0,00",
            ),

            ("workspaceSubView:workspaceForm:workspacePageSubView:removedClientDeletePanelOpenedState", 
                "",
            ),
            ("workspaceSubView:workspaceForm", "workspaceSubView:workspaceForm"),
            ("autoScroll", ""),

            ("javax.faces.ViewState", "j_id1"),
            (
                "workspaceSubView:workspaceForm:workspacePageSubView:clientListTable:j_id_jsp_635818149_104pc51", 
                "next",
            ),
            (
                "ajaxSingle",	
                "workspaceSubView:workspaceForm:workspacePageSubView:clientListTable:j_id_jsp_635818149_104pc51",
            ),
            ("AJAX:EVENTS_COUNT", "1"),
            
            ("workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51", "workspaceSubView:workspaceForm:workspacePageSubView:j_id_jsp_635818149_53pc51"),
        ];

        let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(next))
        .send()
        .await
        .unwrap();

        res = resp.text().await.unwrap();
    }

    let res = serde_json::to_string(&clienst_list).unwrap();
    fs::write("all.json", &res).await.unwrap();

    "complete".to_string()
}
