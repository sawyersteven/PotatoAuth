use actix_web::{web::Bytes, HttpRequest, HttpResponse};
use http::StatusCode;
use serde::Deserialize;

use crate::{
    routes::{parse_post_body, rpc::rpc_response, simple_response},
    sessions::SessionStore,
    shared_data::Sharable,
    userdb::{AcctType, UserDB},
};

#[derive(Deserialize)]
struct Args {
    pub name: String, // used for id only, cannot change
    pub password: Option<String>,
    pub paths: Option<Vec<String>>,
    pub acct_type: Option<AcctType>,
}

pub async fn post(req: HttpRequest, body: Bytes) -> HttpResponse {
    let args: Args = match parse_post_body(body) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Bad post body: {}", e);
            return simple_response(StatusCode::BAD_REQUEST);
        }
    };

    let mut user_db_w = UserDB::extract_from(&req).write().unwrap();

    let user = match user_db_w.get(&args.name) {
        Some(u) => u,
        None => return rpc_response(false, format!("User {} does not exist in database", args.name)),
    };

    match user
        .write()
        .unwrap()
        .update_info(args.password, args.paths, args.acct_type)
    {
        Ok(_) => {}
        Err(e) => return rpc_response(false, e.message),
    }

    SessionStore::extract_from(&req).write().unwrap().remove_for_user(&user);

    match user_db_w.write_to_file() {
        Ok(_) => {}
        Err(e) => {
            return rpc_response(
                false,
                format!("User updated but could not be written to the database: {}", e),
            )
        }
    }

    return rpc_response(true, format!("{} updated", args.name));
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use serde::Deserialize;

    use crate::{
        app::tests::start_test_server,
        config::UserConfig,
        routes::tests::{login_client, make_client, make_test_userdb, send_get, send_post},
        userdb::{AcctType, SafeSerializableUser},
    };

    #[tokio::test]
    async fn post_update_user() -> crate::Result<()> {
        const PORT: u16 = 8666;
        let url = format!("http://localhost:{}/rpc/updateuser", PORT);

        let udb = make_test_userdb();
        let mut cfg = UserConfig::default();
        cfg.user_db = Some(udb);
        cfg.port = Some(PORT);
        start_test_server(cfg);

        let payload = serde_json::json!({"name": "Admin_user", "password": "a_new_password", "paths": ["/hi/mom"], "acct_type": "Admin"});
        let bad_payload =
            serde_json::json!({"name": "User_user", "password": "short", "paths": ["*"], "acct_type": "Admin"});

        let client = make_client();
        login_client(&client, PORT, AcctType::Admin).await;

        let resp = send_post(&client, &url, &bad_payload).await;
        assert!(resp.text().await.unwrap().contains("false"));

        let resp = send_post(&client, &url, &payload).await;
        assert!(resp.text().await.unwrap().contains("true"));

        // try logging in with new Admin_user password
        let client = make_client();
        let resp = client
            .post(format!("http://localhost:{}/login", PORT))
            .json(&serde_json::json!({"username": "Admin_user", "password": "a_new_password"}))
            .send()
            .await
            .expect("Can't send login post");
        assert_eq!(resp.status(), StatusCode::OK);

        // list safe users and verify non-password changes
        let list_url = format!("http://localhost:{}/rpc/listusers", PORT);
        let resp = send_get(&client, &list_url).await;

        #[derive(Deserialize)]
        struct Response {
            ok: bool,
            response: Vec<SafeSerializableUser>,
        }

        let list = match resp.json::<Response>().await {
            Ok(l) => l,
            Err(e) => return Err(crate::Error::convert(e)),
        };

        assert!(list.ok);

        list.response.iter().filter(|u| u.name == "Admin_user").for_each(|u| {
            assert_eq!(u.paths, vec!["/hi/mom"]);
            assert_eq!(u.acct_type, AcctType::Admin);
        });

        return Ok(());
    }
}
