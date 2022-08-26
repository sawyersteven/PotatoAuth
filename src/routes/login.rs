use actix_web::{http::StatusCode, web, HttpRequest, HttpResponse, HttpResponseBuilder};
use serde::Deserialize;

use crate::{
    routes::{parse_post_body, serve_file, simple_response},
    sessions::SessionStore,
    shared_data::Sharable,
    userdb::UserDB,
};

pub async fn get(req: HttpRequest) -> HttpResponse {
    let mut sessions_w = SessionStore::extract_from(&req).write().unwrap();
    if sessions_w.get_from_request(&req).is_some() {
        // if logged in, redirect
        let resp = HttpResponse::Found().append_header(("Location", "/")).finish();
        tracing::warn!("{}", resp.status());
        return resp;
    }
    return serve_file("./static/login.html").await;
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

pub async fn post(req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let sessions = SessionStore::extract_from(&req);

    if sessions.write().unwrap().get_from_request(&req).is_some() {
        // don't start new session if already logged in
        return simple_response(StatusCode::OK);
    }

    let user_db = UserDB::extract_from(&req).read().unwrap();

    let form: LoginForm = match parse_post_body(body) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("{}", e);
            return simple_response(StatusCode::BAD_REQUEST);
        }
    };

    let origin_addr = match req.peer_addr() {
        Some(a) => a.to_string(),
        None => "???".to_string(),
    };

    let resp: HttpResponse;
    match user_db.verify_credentials(&form.username, &form.password) {
        Some(u) => {
            tracing::info!(
                "Login attempt successful for {} from {}",
                u.read().unwrap().get_name(),
                origin_addr
            );
            let mut sessions_w = sessions.write().unwrap();
            let sess = sessions_w.new_session(&u);
            resp = HttpResponseBuilder::new(StatusCode::OK).cookie(sess.cookie()).finish();
        }
        None => {
            tracing::warn!("Login attempt failed for {} from {}", &form.username, origin_addr);
            resp = simple_response(StatusCode::UNAUTHORIZED);
        }
    };
    return resp;
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use crate::{
        app,
        config::UserConfig,
        routes::tests::{make_client, make_test_userdb, send_get, send_post},
    };

    #[tokio::test]
    async fn post_login() {
        const PORT: u16 = 8671;
        let url = format!("http://localhost:{}/login", PORT);

        let mut cfg = UserConfig::default();
        cfg.user_db = Some(make_test_userdb());
        cfg.port = Some(PORT);

        app::tests::start_test_server(cfg);

        let client = make_client();

        let resp = send_post(
            &client,
            &url,
            &serde_json::json!({"invalid": "json", "password": "not_a_password" }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let resp = send_post(
            &client,
            &url,
            &serde_json::json!({"username": "Admin_user", "password": "not_a_password" }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let resp = send_post(
            &client,
            &url,
            &serde_json::json!({"username": "Admin_user", "password": "password" }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = send_get(&client, &url).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        /* ^^^ this is 404 because it redirects to '/', which doesn't exist */
    }
}
