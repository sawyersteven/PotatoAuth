use actix_web::{http::StatusCode, web::Path, HttpResponse};

use super::{serve_file, simple_response};

const FS_DIR: &str = "./static/";

pub async fn get(path: Path<String>) -> HttpResponse {
    let file = path.into_inner();
    if file.ends_with(".html") {
        return simple_response(StatusCode::NOT_FOUND);
    }

    let local_file = format!("{}{}", FS_DIR, file);
    return serve_file(local_file).await;
}
