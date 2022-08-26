use std::{
    path::Path,
    sync::{Arc, RwLock},
};

use actix_web::{
    http::StatusCode,
    web::{Buf, Bytes},
    HttpRequest, HttpResponse, HttpResponseBuilder,
};

use serde::de::DeserializeOwned;

use crate::{sessions::SessionStore, shared_data::Sharable, userdb::User};

pub mod admin;
pub mod auth_request;
pub mod login;
pub mod logout;
pub mod rpc;
pub mod setup;
pub mod static_dir;

pub const SESSION_NAME: &str = "potato_session";

#[inline]
pub fn simple_response(status_code: StatusCode) -> HttpResponse {
    return HttpResponseBuilder::new(status_code).body(status_code.to_string());
}

fn parse_post_body<T>(body: Bytes) -> crate::Result<T>
where
    T: DeserializeOwned,
{
    match serde_json::from_reader(body.reader()) {
        Ok(a) => return Ok(a),
        Err(e) => return Err(crate::Error::convert(e)),
    };
}

fn req_user(req: &HttpRequest) -> Option<Arc<RwLock<User>>> {
    let mut sessions_w = SessionStore::extract_from(&req).write().unwrap();
    return Some(sessions_w.get_from_request(&req)?.get_user().clone());
}

#[inline]
pub async fn serve_file(filepath: impl AsRef<Path>) -> HttpResponse {
    return match tokio::fs::read(filepath).await {
        Ok(contents) => HttpResponse::Ok().body(contents),
        Err(_) => simple_response(StatusCode::NOT_FOUND),
    };
}

/// Clears session data from req
pub fn remove_session(req: &HttpRequest) {
    let cookie = match req.cookie(SESSION_NAME) {
        Some(c) => c,
        None => return,
    };
    SessionStore::extract_from(&req)
        .write()
        .unwrap()
        .remove_id(&cookie.value().to_string());
}

#[cfg(test)]
pub mod tests {
    use http::StatusCode;
    use reqwest::{Client, Response};

    use crate::{test_utils::make_tmp_file, userdb::AcctType};

    pub fn make_client() -> reqwest::Client {
        return reqwest::ClientBuilder::new()
            .cookie_store(true)
            .build()
            .expect("Can't build client");
    }

    /// Make user db with admin_user:password and user_user:password logins
    pub fn make_test_userdb() -> String {
        let udb = make_tmp_file();
        std::fs::write(&udb, "Admin_user:$argon2i$v=19$m=16,t=2,p=1$Z3FyNTJoUlFZSkFPZE80TA$uOHPaLj1dPwVuStA8SlpXA:*:Admin\nUser_user:$argon2i$v=19$m=16,t=2,p=1$Z3FyNTJoUlFZSkFPZE80TA$uOHPaLj1dPwVuStA8SlpXA:*:User").expect("Can't make userdb");
        return udb;
    }

    pub async fn login_client(client: &Client, test_port: u16, acct_type: AcctType) {
        let resp = client
            .post(format!("http://localhost:{}/login", test_port))
            .json(&serde_json::json!({"username": format!("{:#?}_user", acct_type), "password": "password"}))
            .send()
            .await
            .expect("Can't send login post");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    pub async fn send_post(client: &Client, url: &String, payload: &serde_json::Value) -> Response {
        return client
            .post(url)
            .json(payload)
            .send()
            .await
            .expect("Can't send post request");
    }

    pub async fn send_get(client: &Client, url: &String) -> Response {
        return client.get(url).send().await.expect("Can't send get request");
    }
}
