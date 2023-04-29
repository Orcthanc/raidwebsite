use std::ops::Deref;

use actix_session::{Session, SessionExt};
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::{ContentType, LOCATION};
use actix_web::{get, post, web, HttpResponse, Responder, HttpMessage};
use actix_web_lab::middleware::Next;
use sqlx::{MySqlPool, query_as};
use log::error;

use crate::crypto::{argon2_hash_text, argon2_verify_password};
use crate::data::User;

#[get("/register")]
async fn register_form() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("register_form.html"))
}

#[post("/register")]
async fn register(
    form: web::Form<User>,
    pool: web::Data<MySqlPool>,
) -> impl Responder {

    let trans = pool.get_ref().begin().await;

    let mut trans = match trans {
        Ok(t) => t,
        Err(e) => panic!("{}", e),
    };

    let hash = argon2_hash_text(&form.password);

    let hash = if let Ok(s) = hash {
        s
    } else {
        error!("{:?}", hash);
        return HttpResponse::BadRequest().body("Invalid password");
    };

    let result = query_as!(
        User,
        "INSERT INTO users (username, password_hash) VALUES (?, ?)",
        form.username,
        hash,
    )
    .execute(&mut trans)
    .await;
    match result {
        Ok(_) => { 
            if let Err(e) = trans.commit().await{
                error!("{:?}", e);
                return HttpResponse::InternalServerError().finish();
            }
            return HttpResponse::Ok().body("User registered successfully");
        },
        Err(e) => {
            return match e {
                sqlx::Error::Database(_) => HttpResponse::BadRequest().body("User already exists"),
                _ => {
                    error!("{:?}", e);
                    return HttpResponse::InternalServerError().finish();
                },
            }
        },
    }
}

#[get("/login")]
async fn login_form() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("login_form.html"))
}

#[post("/login")]
async fn login(
    form: web::Form<User>,
    pool: web::Data<MySqlPool>,
    session: Session,
) -> impl Responder {
    let trans = pool.get_ref().begin().await;

    let mut trans = match trans {
        Ok(t) => t,
        Err(e) => panic!("{}", e),
    };

    let p_hash = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE username = ?",
        form.username,
    )
    .fetch_one(&mut trans)
    .await;

    let (uid, p_hash) = match p_hash {
        Ok(s) => (s.id, s.password_hash),
        Err(_) => return HttpResponse::BadRequest().body("Invalid Username or Password"),
    };

    match argon2_verify_password(&form.password, &p_hash){
        Ok(_) => {
            if let Err(_) = session.insert("id", uid){
                return HttpResponse::InternalServerError().finish();
            }
            return HttpResponse::SeeOther()
                .insert_header((LOCATION, "/auth/me/chars"))
                .finish();
        },
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::BadRequest().body("Invalid Username or Password");
        }
    };
}

#[post("/logout")]
async fn logout(
    session: Session,
) -> impl Responder {
    session.purge();
    return HttpResponse::Ok().body("Successfully logged out");
}

#[derive(Copy, Clone, Debug)]
pub struct UserId(i32);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum WebError {
    NotLoggedIn,
}

impl std::fmt::Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            WebError::NotLoggedIn => write!(f, "User is not logged in"),
        }
    }
}

pub async fn reject_unauth_user(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = req.parts().0.get_session();

    match session.get::<i32>("id").map_err(actix_web::error::ErrorInternalServerError)? {
        Some(uid) => {
            req.extensions_mut().insert(UserId(uid));
            next.call(req).await
        },
        None => {
            Err(actix_web::error::InternalError::from_response(
                    WebError::NotLoggedIn,
                    HttpResponse::SeeOther().insert_header((LOCATION, "/login")).finish()).into())
        }
    }
}
