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

    // fs::write("foo.html", resp.text().await.unwrap())
    //     .await
    //     .unwrap();

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

    // fs::write("foo1.html", resp.text().await.unwrap())
    //     .await
    //     .unwrap();

    let mut org_html = resp.text().await.unwrap();
    if !check_first_org(&org_html){
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
        if !check_next_org_page(&html){
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

 fn check_first_org(org_html: &String) -> bool{
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
            //org_html.clone()
        } ,
        _ => {
            dbg!(td.inner_text(parser).to_string().as_str());
            //let n = executor::block_on(click_on_org_page(4, client));
            false
        },
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
        return false
    }
    true 
        
}

fn button_table_dom(org_html: &String) -> String{
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