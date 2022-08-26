use actix_web::{http::StatusCode, web::Bytes, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::{
    routes::{parse_post_body, simple_response},
    shared_data::Sharable,
    userdb::{AcctType, UserDB},
};

use super::rpc_response;

#[derive(Deserialize)]
pub struct Args {
    pub name: String,
    pub password: String,
    pub paths: Vec<String>,
    pub acct_type: AcctType,
}

pub async fn post(req: HttpRequest, body: Bytes) -> HttpResponse {
    let mut args: Args = match parse_post_body(body) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Bad post body: {}", e);
            return simple_response(StatusCode::BAD_REQUEST);
        }
    };

    args.paths = args.paths.iter().map(|p| p.trim().to_string()).collect();

    let resp = match UserDB::extract_from(&req).write().unwrap().add_user(
        &args.name,
        &args.password,
        &args.paths,
        args.acct_type,
    ) {
        Ok(_) => rpc_response(true, format!("{} account added", args.name)),
        Err(e) => rpc_response(false, e.message),
    };
    return resp;
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use crate::{
        app::tests::start_test_server,
        config::UserConfig,
        routes::tests::{login_client, make_client, make_test_userdb, send_post},
        userdb::AcctType,
    };

    #[tokio::test]
    async fn post_add_user() {
        const PORT: u16 = 8669;
        let url = format!("http://localhost:{}/rpc/adduser", PORT);

        let udb = make_test_userdb();
        let mut cfg = UserConfig::default();
        cfg.user_db = Some(udb);
        cfg.port = Some(PORT);
        start_test_server(cfg);

        let payload = serde_json::json!({"name": "new_user", "password": "new_password", "paths": ["*"], "acct_type": "User"});
        let bad_payload =
            serde_json::json!({"name": "new_user", "password": "short", "paths": ["*"], "acct_type": "User"});

        let client = make_client();
        login_client(&client, PORT, AcctType::Admin).await;

        let resp = send_post(&client, &url, &bad_payload).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.text().await.unwrap().contains("false"));

        let resp = send_post(&client, &url, &payload).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.text().await.unwrap().contains("true"));

        // Try logging in with new acct
        let client = make_client();
        assert_eq!(
            client
                .post(format!("http://localhost:{}/login", PORT))
                .json(&serde_json::json!({"username":"new_user", "password": "new_password"}))
                .send()
                .await
                .expect("can't send post request")
                .status(),
            StatusCode::OK
        );
    }
}
