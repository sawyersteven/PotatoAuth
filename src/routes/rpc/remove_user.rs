use std::sync::{Arc, RwLock};

use actix_web::{http::StatusCode, web::Bytes, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::{
    routes::{parse_post_body, rpc::rpc_response, simple_response},
    sessions::SessionStore,
    shared_data::Sharable,
    userdb::{User, UserDB},
};

#[derive(Deserialize)]
struct Args {
    username: String,
}

pub async fn post(req: HttpRequest, body: Bytes, user: Arc<RwLock<User>>) -> HttpResponse {
    let mut user_db = UserDB::extract_from(&req).write().unwrap();

    let args: Args = match parse_post_body(body) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Bad post body: {}", e);
            return simple_response(StatusCode::BAD_REQUEST);
        }
    };

    if user.read().unwrap().get_name() == &args.username {
        return rpc_response(false, "Cannot remove your own account");
    }

    SessionStore::extract_from(&req)
        .write()
        .unwrap()
        .remove_id(&args.username);

    return match user_db.remove(&args.username) {
        Ok(_) => rpc_response(true, format!("User {} removed", &args.username)),
        Err(e) => rpc_response(false, e.message),
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        app::tests::start_test_server,
        config::UserConfig,
        routes::tests::{login_client, make_client, make_test_userdb, send_post},
        userdb::AcctType,
    };

    #[tokio::test]
    async fn post_remove_user() {
        const PORT: u16 = 8667;
        let url = format!("http://localhost:{}/rpc/removeuser", PORT);

        let udb = make_test_userdb();
        let mut cfg = UserConfig::default();
        cfg.user_db = Some(udb);
        cfg.port = Some(PORT);
        start_test_server(cfg);

        let bad_payload = serde_json::json!({"username": "Admin_user"});
        let payload = serde_json::json!({"username": "User_user"});

        let client = make_client();
        login_client(&client, PORT, AcctType::Admin).await;

        let resp = send_post(&client, &url, &bad_payload).await;
        assert!(resp.text().await.unwrap().contains("false"));

        let resp = send_post(&client, &url, &payload).await;
        assert!(resp.text().await.unwrap().contains("true"));
    }
}
