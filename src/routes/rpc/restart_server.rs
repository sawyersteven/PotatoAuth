use std::time::Duration;

use actix_web::{http::StatusCode, web::Bytes, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::{
    app::{ExitCommand, ServerController},
    routes::{parse_post_body, rpc::rpc_response, simple_response},
    shared_data::Sharable,
};

#[derive(Deserialize)]
struct Args {
    command: ExitCommand,
}

pub async fn post(req: HttpRequest, body: Bytes) -> HttpResponse {
    let args: Args = match parse_post_body(body) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("{}", e);
            return simple_response(StatusCode::BAD_REQUEST);
        }
    };

    let signaller = ServerController::extract_from(&req).clone();
    tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        _ = signaller.send_exit(args.command, false).await;
    });

    return rpc_response(true, "");
}

#[cfg(test)]
pub mod tests {
    use http::StatusCode;

    use crate::app;
    use crate::config::UserConfig;
    use crate::routes::tests::{login_client, make_client, make_test_userdb, send_post};
    use crate::userdb::AcctType;

    #[tokio::test]
    async fn post_restart() {
        const PORT: u16 = 8665;
        let url = format!("http://localhost:{}/rpc/restartserver", PORT);

        let mut cfg = UserConfig::default();
        cfg.user_db = Some(make_test_userdb());
        cfg.port = Some(PORT);

        app::tests::start_test_server(cfg);

        let client = make_client();
        login_client(&client, PORT, AcctType::Admin).await;

        let resp = send_post(&client, &url, &serde_json::json!({"command": "NaN"})).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let resp = send_post(&client, &url, &serde_json::json!({"command": "Restart"})).await;
        assert_eq!(resp.status(), StatusCode::OK);

        /* For reasons I don't entirely understand, sending the Restart command
        from a reqwest request causes the server function to not return and
        handle the exit_command. So this test only checks that a bad request
        doesn't do anything.
        */
    }
}
