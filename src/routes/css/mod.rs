use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::get;
use actix_web::http::header::ContentType;
use mime;

#[get("styles.css")]
pub async fn base_styles() -> impl Responder{
    HttpResponse::Ok()
        .insert_header(ContentType(mime::TEXT_CSS))
        .body(include_str!("styles.css"))
}

#[get("login_style.css")]
pub async fn login_style() -> impl Responder{
    HttpResponse::Ok()
        .insert_header(ContentType(mime::TEXT_CSS))
        .body(include_str!("login_style.css"))
}

#[get("character_style.css")]
pub async fn character_style() -> impl Responder{
    HttpResponse::Ok()
        .insert_header(ContentType(mime::TEXT_CSS))
        .body(include_str!("character_style.css"))
}

