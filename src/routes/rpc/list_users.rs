use actix_web::{HttpRequest, HttpResponse};

use crate::{routes::rpc::rpc_response, shared_data::Sharable, userdb::UserDB};

pub async fn get(req: HttpRequest) -> HttpResponse {
    let user_db = UserDB::extract_from(&req).read().unwrap();

    return rpc_response(true, user_db.list_safe());
}
