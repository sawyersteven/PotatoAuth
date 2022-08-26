use actix_web::HttpRequest;

use crate::userdb::AcctType;

use super::*;

pub async fn get(req: HttpRequest) -> HttpResponse {
    return match req_user(&req) {
        Some(usr) => {
            if usr.read().unwrap().get_type() == &AcctType::Admin {
                serve_file("./static/admin.html").await
            } else {
                simple_response(StatusCode::UNAUTHORIZED)
            }
        }
        None => login::get(req).await,
    };
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use crate::{
        app::tests::start_test_server,
        config::UserConfig,
        routes::tests::{login_client, make_client, make_test_userdb},
        userdb::AcctType,
    };

    #[tokio::test]
    async fn get_admin() {
        const PORT: u16 = 8672;

        let udb = make_test_userdb();
        let mut cfg = UserConfig::default();
        cfg.port = Some(PORT);
        cfg.user_db = Some(udb);
        start_test_server(cfg);

        let client = make_client();
        login_client(&client, PORT, AcctType::User).await;

        let resp = client
            .get(format!("http://localhost:{}/admin", PORT))
            .send()
            .await
            .expect("Can't send get request");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let client = make_client();
        login_client(&client, PORT, AcctType::Admin).await;

        let resp = client
            .get(format!("http://localhost:{}/admin", PORT))
            .send()
            .await
            .expect("Can't send get request");
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
