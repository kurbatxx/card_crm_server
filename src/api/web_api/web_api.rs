#[path = "post.rs"]
mod post;

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

const TEMP: &str = "temp";

enum Cards {
    NoSelected,
    WithCards,
    NoCards,
}

impl Cards {
    fn value(&self) -> &str {
        match *self {
            Cards::NoSelected => "0",
            Cards::WithCards => "1",
            Cards::NoCards => "2",
        }
    }
}

#[derive(Debug, Deserialize)]
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
        .form(&HashMap::from(post::LOGOUT))
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
    if cfg!(debug_assertions) {
        fs::create_dir_all("./".to_owned() + TEMP).await.unwrap();
    }

    dbg!(&payload);
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
    let resp = web_client
        .client
        .post(SITE_URL)
        .form(&HashMap::from(post::CLICK_CLIENTS_LIST))
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

    let _resp = client
        .post(SITE_URL)
        .form(&HashMap::from(post::ORG_MODAL))
        .send()
        .await
        .unwrap();

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(post::CLICK_OPEN_ORG_POPUP))
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
    let number = number.to_string();

    let mut click_org_page = HashMap::from(post::CLICK_ORG_PAGE);
    click_org_page
        .entry(post::CLICK_ORG_PAGE_KEY)
        .and_modify(|e| *e = &number);

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
    pub org_id: i32,
    pub show_deleted: bool,
}

pub async fn init_search(
    Extension(clients): Extension<Clients>,
    Query(query): Query<Name>,
    extract::Json(payload): extract::Json<SearchQuery>,
) -> String {
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

    //**
    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(post::CLEAR_BUTTON))
        .send()
        .await
        .unwrap();

    if cfg!(debug_assertions) {
        fs::write("1_clear_button.html", resp.text().await.unwrap())
            .await
            .unwrap();
    }

    let mut search_form = HashMap::from(post::SEARCH_FORM);

    let id_str = id.to_string();
    if id != 0 {
        search_form
            .entry(post::SEARCH_ID_KEY)
            .and_modify(|e| *e = &id_str);
    } else {
        if payload.org_id != 0 {
            let org_id = payload.org_id.to_string();
            search_form = set_org_filter(&org_id, search_form, &client).await;
        }

        if payload.show_deleted {
            search_form.insert(post::SHOW_DELETED_KEY, "on");
        } else {
            search_form.remove(post::SHOW_DELETED_KEY);
        }

        search_form
            .entry(post::LAST_NAME_KEY)
            .and_modify(|e| *e = &full_name.last_name);

        search_form
            .entry(post::NAME_KEY)
            .and_modify(|e| *e = &full_name.name);

        search_form
            .entry(post::SURNAME_KEY)
            .and_modify(|e| *e = &full_name.surname);
    }

    let resp = client
        .post(SITE_URL)
        .form(&search_form)
        .send()
        .await
        .unwrap();

    let res = resp.text().await.unwrap();
    if cfg!(debug_assertions) {
        fs::write(TEMP.to_owned() + "/6_search_submit.html", &res)
            .await
            .unwrap();
    }

    let org_client_page = res;
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
    pub fullname: FullName,
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
                fullname: get_fullname(cells[3].to_string()),
                group: cells[4].to_string(),
                org: cells[6].to_string(),
                balance: cells[7].to_string(),
            };

            org_client
        })
        .collect();

    (org_clients_page, next_page_exist)
}
#[derive(Serialize)]
pub struct FullName {
    last_name: String,
    name: String,
    surname: String,
}

fn get_fullname(fullname: String) -> FullName {
    let mut full_name = FullName {
        name: "".to_string(),
        last_name: "".to_string(),
        surname: "".to_string(),
    };

    let fullname = fullname.trim();
    let arr: Vec<_> = fullname.split_whitespace().collect();

    match arr.len() {
        1 => full_name.last_name = arr[0].to_string(),
        2 => {
            full_name.last_name = arr[0].to_string();
            full_name.name = arr[1].to_string();
        }
        3.. => {
            full_name.last_name = arr[0].to_string();
            full_name.name = arr[1].to_string();

            let sur = &arr[2..];
            let surname = sur.join(" ");
            full_name.surname = surname;
        }
        _ => {}
    }

    return full_name;
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

    let mut click_bottom_number = HashMap::from(post::CLICK_SEARCH_BOTTOM_NUMBER);

    click_bottom_number
        .entry(post::BOTTOM_NUMBER_KEY)
        .and_modify(|e| *e = &query.page_number);

    let resp = client
        .post(SITE_URL)
        .form(&click_bottom_number)
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

#[derive(Deserialize, Debug, Clone)]
pub struct FullDownloadQuery {
    pub org_id: i32,
    pub cards: i32,
    pub show_deleted: bool,
}

pub async fn download_all(
    Extension(clients): Extension<Clients>,
    Query(query): Query<Name>,
    extract::Json(payload): extract::Json<FullDownloadQuery>,
) -> String {
    let web_client = create_client_or_send_exist(&query.login, &clients).await;

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await.not() {
            return "Не авторизован".to_string();
        }
    }

    //web_client.search_query = Some(payload.clone());
    // clients
    //     .write()
    //     .await
    //     .insert(query.login.clone(), web_client);

    // let web_client = create_client_or_send_exist(&query.login, &clients).await;
    // dbg!(&web_client.search_query);

    let client = web_client.client;
    let _ = client.get(SITE_URL).send().await.unwrap();

    let resp = client
        .post(SITE_URL)
        .form(&HashMap::from(post::CLEAR_BUTTON))
        .send()
        .await
        .unwrap();

    if cfg!(debug_assertions) {
        fs::write(
            TEMP.to_owned() + "/1_clear_button.html",
            resp.text().await.unwrap(),
        )
        .await
        .unwrap();
    }

    let mut search_form = HashMap::from(post::SEARCH_FORM);

    if payload.org_id == 0 {
        return "Не выбрана организация".to_string();
    }

    let org_id = payload.org_id.to_string();
    search_form = set_org_filter(&org_id, search_form, &client).await;

    let card_selector = payload.cards.to_string();
    search_form
        .entry(post::CARDS_KEY)
        .and_modify(|e| *e = &card_selector);

    let resp = client
        .post(SITE_URL)
        .form(&search_form)
        .send()
        .await
        .unwrap();

    let mut res = resp.text().await.unwrap();
    if cfg!(debug_assertions) {
        fs::write(TEMP.to_owned() + "/6_search_submit.html", &res)
            .await
            .unwrap();
    }

    //Первая страница с фильтром..

    let mut clienst_list: Vec<OrgClient> = vec![];

    search_form.insert(post::NEXT_SEARCH_PAGE_KEY, "next");
    search_form.insert("ajaxSingle", post::NEXT_SEARCH_PAGE_KEY);

    loop {
        let (part_clients, next_page_exist) = parse_clients_page(&res);
        clienst_list.extend(part_clients);
        if next_page_exist.not() {
            break;
        }

        let resp = client
            .post(SITE_URL)
            .form(&search_form)
            .send()
            .await
            .unwrap();

        res = resp.text().await.unwrap();
    }

    let res = serde_json::to_string(&clienst_list).unwrap();
    fs::write("all.json", &res).await.unwrap();

    "complete".to_string()
}

async fn set_org_filter<'a>(
    org_id: &str,
    mut search_form: HashMap<&'a str, &'a str>,
    client: &Client,
) -> HashMap<&'a str, &'a str> {
    search_form.remove(post::SUBMIT_SEARCH_KEY);
    search_form.insert(post::OPEN_ORG_SEARCH_KEY, post::OPEN_ORG_SEARCH_KEY);

    let resp = client
        .post(SITE_URL)
        .form(&search_form)
        .send()
        .await
        .unwrap();

    search_form.remove(post::OPEN_ORG_SEARCH_KEY);

    if cfg!(debug_assertions) {
        fs::write(
            TEMP.to_owned() + "/2_open_org_selector.html",
            resp.text().await.unwrap(),
        )
        .await
        .unwrap();
    }

    let mut set_filter = HashMap::from(post::ORG_FILTER);
    set_filter
        .entry(post::ORG_FILTER_BY_ID_KEY)
        .and_modify(|e| *e = &org_id);

    let resp = client
        .post(SITE_URL)
        .form(&set_filter)
        .send()
        .await
        .unwrap();

    if cfg!(debug_assertions) {
        fs::write(
            TEMP.to_owned() + "/3_set_filter.html",
            resp.text().await.unwrap(),
        )
        .await
        .unwrap();
    }

    set_filter.remove(post::ORG_MODAL_FILTER_KEY);
    set_filter.remove(post::AJAX_EVENTS_COUNT_KEY);

    set_filter.insert(post::FIRST_TABLE_ROW_KEY, post::FIRST_TABLE_ROW_KEY);

    let resp = client
        .post(SITE_URL)
        .form(&set_filter)
        .send()
        .await
        .unwrap();

    if cfg!(debug_assertions) {
        fs::write(
            TEMP.to_owned() + "/4_click_table_element.html",
            resp.text().await.unwrap(),
        )
        .await
        .unwrap();
    }

    set_filter.remove(post::FIRST_TABLE_ROW_KEY);
    set_filter.insert(post::SUBMIT_SELECTED_ROW, post::SUBMIT_SELECTED_ROW);

    let resp = client
        .post(SITE_URL)
        .form(&set_filter)
        .send()
        .await
        .unwrap();

    if cfg!(debug_assertions) {
        fs::write(
            TEMP.to_owned() + "/5_submit_selected_row.html",
            resp.text().await.unwrap(),
        )
        .await
        .unwrap();
    }

    search_form.insert(post::AJAX_EVENTS_COUNT_KEY, "1");
    search_form.insert(post::SUBMIT_SEARCH_KEY, post::SUBMIT_SEARCH_KEY);

    search_form
}

#[derive(Deserialize, Debug, Clone)]
pub struct QueryPerson {
    login: String,
    num: u32,
}

pub async fn person_info(
    Extension(clients): Extension<Clients>,
    Query(query): Query<QueryPerson>,
) -> String {
    let web_client = create_client_or_send_exist(&query.login, &clients).await;

    if web_client.cookie.is_empty().not() {
        if check_auth(&web_client).await.not() {
            return "Не авторизован".to_string();
        }
    }

    let client = web_client.client;

    let mut search_form = HashMap::from(post::SEARCH_FORM);
    search_form.remove(post::SUBMIT_SEARCH_KEY);

    let table_row_key = format!("workspaceSubView:workspaceForm:workspacePageSubView:clientListTable:{}:j_id_jsp_635818149_64pc51", &query.num);
    search_form.insert(&table_row_key, &table_row_key);

    let resp = client
        .post(SITE_URL)
        .form(&search_form)
        .send()
        .await
        .unwrap();

    if cfg!(debug_assertions) {
        fs::write("person.html", resp.text().await.unwrap())
            .await
            .unwrap();
    }

    //TODO: PARSE person info

    let mut undo = HashMap::from(post::LIST_UNDO);

    // let mut undo = HashMap::from(post::CLICK_CLIENTS_LIST);
    // undo.insert(
    //     "panelMenuStatemainMenuSubView:mainMenuForm:selectedClientGroupMenu",
    //     "opened",
    // );

    // undo.insert(
    //     "panelMenuStatemainMenuSubView:mainMenuForm:cardGroupMenu",
    //     "opened",
    // );

    // undo.insert(
    //     "panelMenuStatemainMenuSubView:mainMenuForm:selectedCardGroupMenu",
    //     "opened",
    // );

    //UNDO
    let resp = client.post(SITE_URL).form(&undo).send().await.unwrap();
    if cfg!(debug_assertions) {
        fs::write("undo.html", resp.text().await.unwrap())
            .await
            .unwrap();
    }
    "complete".to_string()
}
