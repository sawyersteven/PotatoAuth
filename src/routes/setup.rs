use std::time::Duration;

use actix_web::{
    http::StatusCode,
    web::{self},
    HttpRequest, HttpResponse,
};
use serde::Deserialize;

use crate::{
    app::{ExitCommand, ServerController},
    routes::simple_response,
    shared_data::Sharable,
    userdb::{AcctType, UserDB},
};

use super::{parse_post_body, serve_file};

pub async fn get(_req: HttpRequest) -> HttpResponse {
    return serve_file("./static/setup.html").await;
}

#[derive(Deserialize)]
struct Args {
    name: String,
    password: String,
}

pub async fn post(req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let args = match parse_post_body::<Args>(body) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Body post body sent to /setup: {}", e.message);
            return simple_response(StatusCode::BAD_REQUEST);
        }
    };

    let resp = match UserDB::extract_from(&req).write().unwrap().add_user(
        &args.name,
        &args.password,
        &vec!["*".to_string()],
        AcctType::Admin,
    ) {
        Ok(_) => simple_response(StatusCode::OK),
        Err(e) => {
            tracing::error!("{}", e);
            return simple_response(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let signaller = ServerController::extract_from(&req).clone();
    tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        _ = signaller.send_exit(ExitCommand::Restart, false).await;
    });

    return resp;
}

#[cfg(test)]
pub mod tests {
    use crate::app;
    use crate::config::UserConfig;
    use crate::routes::tests::{make_client, send_post};
    use crate::test_utils::make_tmp_file;
    use http::StatusCode;

    #[tokio::test]
    async fn post_setup() {
        const PORT: u16 = 8664;
        let url = format!("http://localhost:{}/setup", PORT);

        let mut cfg = UserConfig::default();
        cfg.user_db = Some(make_tmp_file());
        cfg.port = Some(PORT);

        app::tests::start_test_server(cfg);

        let client = make_client();
        let resp = send_post(
            &client,
            &url,
            &serde_json::json!({"name": "username", "password": "password"}),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);

        /* See restart_server.rs */
    }
}
