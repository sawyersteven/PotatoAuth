use actix_web::{
    http::StatusCode,
    web::{Bytes, Path},
    HttpRequest, HttpResponse,
};
use serde::Serialize;
use serde_json::json;

mod add_user;
mod list_users;
mod remove_user;
mod restart_server;
mod update_user;

use crate::{routes::simple_response, sessions::SessionStore, shared_data::Sharable, userdb::AcctType};

use super::req_user;

pub async fn get(req: HttpRequest, path: Path<String>) -> HttpResponse {
    if req_user(&req)
        .filter(|u| u.read().unwrap().get_type() == &AcctType::Admin)
        .is_none()
    {
        return simple_response(StatusCode::UNAUTHORIZED);
    }

    return match &*path.into_inner() {
        "listusers" => list_users::get(req).await,
        _ => simple_response(StatusCode::NOT_FOUND),
    };
}

pub async fn post(req: HttpRequest, body: Bytes, path: Path<String>) -> HttpResponse {
    let sessions = SessionStore::extract_from(&req);
    let user = match sessions
        .write()
        .unwrap()
        .get_from_request(&req)
        .and_then(|session| Some(session.get_user()))
        .filter(|u| u.read().unwrap().get_type() == &AcctType::Admin)
    {
        Some(u) => u,
        None => return simple_response(StatusCode::UNAUTHORIZED),
    };

    return match &*path.into_inner() {
        "adduser" => add_user::post(req, body).await,
        "removeuser" => remove_user::post(req, body, user).await,
        "updateuser" => update_user::post(req, body).await,
        "restartserver" => restart_server::post(req, body).await,
        _ => simple_response(StatusCode::NOT_FOUND),
    };
}

fn rpc_response<T>(ok: bool, response: T) -> HttpResponse
where
    T: Serialize,
{
    let ser = json!({
        "ok": ok,
        "response":response
    })
    .to_string();

    return HttpResponse::Ok().body(ser);
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use crate::app::tests::start_test_server;
    use crate::routes::tests::make_client;
    use crate::userdb::AcctType;
    use crate::{
        config::UserConfig,
        routes::tests::{login_client, make_test_userdb, send_get},
    };

    #[tokio::test]
    async fn test_admin_access() {
        const PORT: u16 = 8668;
        let url = format!("http://localhost:{}/rpc/listusers", PORT);

        let udb = make_test_userdb();
        let mut cfg = UserConfig::default();
        cfg.user_db = Some(udb);
        cfg.port = Some(PORT);
        start_test_server(cfg);

        let client = make_client();
        login_client(&client, PORT, AcctType::User).await;

        assert_eq!(send_get(&client, &url).await.status(), StatusCode::UNAUTHORIZED);

        let client = make_client();
        login_client(&client, PORT, AcctType::Admin).await;

        assert_ne!(send_get(&client, &url).await.status(), StatusCode::UNAUTHORIZED);
    }
}
