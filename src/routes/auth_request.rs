use actix_web::{http::StatusCode, HttpRequest, HttpResponse};

use crate::{sessions::SessionStore, shared_data::Sharable};

use super::simple_response;

pub async fn get(req: HttpRequest) -> HttpResponse {
    let mut sessions_w = SessionStore::extract_from(&req).write().unwrap();
    let sess = match sessions_w.get_from_request(&req) {
        Some(s) => s,
        None => return simple_response(StatusCode::UNAUTHORIZED),
    };

    if !sess.get_user().read().unwrap().path_allowed(req.path()) {
        return simple_response(StatusCode::NOT_FOUND);
    }

    return HttpResponse::Ok().finish();
}
