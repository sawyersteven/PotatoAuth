use actix_web::{HttpRequest, HttpResponse};

use super::{remove_session, serve_file};

pub async fn get(req: HttpRequest) -> HttpResponse {
    remove_session(&req);
    return serve_file("./logout.html").await;
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use crate::{
        app::tests::start_test_server,
        config::UserConfig,
        routes::tests::{login_client, make_client, make_test_userdb, send_get},
        userdb::AcctType,
    };

    #[tokio::test]
    async fn get_logout() {
        const PORT: u16 = 8670;
        let url_auth_request = format!("http://localhost:{}/authrequest", PORT);
        let url_logout = format!("http://localhost:{}/logout", PORT);

        let udb = make_test_userdb();
        let mut cfg = UserConfig::default();
        cfg.user_db = Some(udb);
        cfg.port = Some(8670);
        start_test_server(cfg);

        let client = make_client();
        login_client(&client, 8670, AcctType::User).await;

        let resp = send_get(&client, &url_auth_request).await;
        assert_eq!(resp.status(), StatusCode::OK);

        send_get(&client, &url_logout).await;

        let resp = send_get(&client, &url_auth_request).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
