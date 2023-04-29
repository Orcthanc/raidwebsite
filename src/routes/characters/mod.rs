use crate::data::{Character, Class};

use actix_web::{get, post, Responder, HttpResponse, http::header::LOCATION};
use actix_session::Session;
use actix_web::web;
use log::error;
use serde::Serialize;
use sqlx::{MySqlPool, query_as};
use tera::{Tera, Context};

#[derive(Serialize)]
struct CharContext {
    chars: Vec<RenderableChar>,
    name: String,
}

#[derive(Serialize)]
struct RenderableChar {
    name: String,
    class: String,
    item_level: i32,
}

#[get("/me/add_char")]
async fn add_char(
    tera: web::Data<Tera>,
    session: Session,
    pool: web::Data<MySqlPool>,
) -> impl Responder {
    let trans = pool.get_ref().begin().await;

    let mut trans = match trans {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to db"),
    };

    let classes = sqlx::query_as!(
        Class,
        "SELECT * FROM classes"
    ).fetch_all(&mut trans)
    .await
    .unwrap();

    let mut con = Context::new();
    con.insert("uid", &session.get::<i32>("id").unwrap());
    con.insert("classes", &classes);

    return HttpResponse::Ok().body(
        tera.render("add_character.html", &con).unwrap()
    );
}

#[post("/me/add_char")]
async fn post_add_char(
    session: Session,
    pool: web::Data<MySqlPool>,
    chara: web::Form<Character>,
) -> impl Responder {
    let id: i32 = session.get("id").unwrap().unwrap();

    let trans = pool.get_ref().begin().await;

    let mut trans = match trans {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to db"),
    };

    if id != chara.user_id {
        return HttpResponse::BadRequest().body("[softly]<br>Don't");
    }

    let res = match query_as!(
        Character,
        "INSERT INTO characters (user_id, name, class_id, item_level) VALUES (?, ?, ?, ?)",
        chara.user_id,
        chara.name,
        chara.class_id,
        chara.item_level
    ).execute(&mut trans)
    .await {
        Ok(_) => HttpResponse::SeeOther().insert_header((LOCATION, "chars")).finish(),
        Err(_) => HttpResponse::InternalServerError().body("Could not create Character"),
    };

    if let Err(e) = trans.commit().await {
        error!("Database Error: {:?}", e);
        return HttpResponse::InternalServerError().body("Could not write char to database");
    }

    return res;
}

#[get("/me/chars")]
async fn show_chars(
    session: Session,
    pool: web::Data<MySqlPool>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let id: i32 = session.get("id").unwrap().unwrap();

    let trans = pool.get_ref().begin().await;

    let mut trans = match trans {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to db"),
    };

    //TODO maybe non-repeatable read for performance
    let chars = sqlx::query_as!(
        RenderableChar,
        "SELECT ch.name as name, cl.name as class, ch.item_level 
        FROM characters ch 
        JOIN classes cl ON ch.class_id = cl.id
        WHERE ch.user_id = ?",
        id
    )
    .fetch_all(&mut trans).await;

    let name = sqlx::query!(
        "SELECT username FROM users WHERE id = ?",
        id
    ).fetch_one(&mut trans).await;

    let charc = CharContext {
        name: name.unwrap().username,
        chars: match chars {
            Ok(c) => c,
            Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    }};

    if let Err(_) = trans.commit().await {
        error!("Database error");
        return HttpResponse::InternalServerError().body("Database error");
    }

    let html_str = tera.render("characters.html", &Context::from_serialize(&charc).unwrap()).unwrap();

    return HttpResponse::Ok().body(html_str);
}
