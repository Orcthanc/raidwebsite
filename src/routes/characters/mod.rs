use crate::data::{Character, Class};

use actix_web::{get, post, Responder, HttpResponse, http::header::LOCATION};
use actix_session::Session;
use actix_web::web;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, query_as};
use tera::{Tera, Context};
use itertools::izip;

#[derive(Debug, Serialize)]
struct CharContext {
    activities: Vec<Activity>,
    chars: Vec<CompleteChar>,
    name: String,
}

#[derive(Serialize)]
struct RenderableChar {
    id: i32,
    name: String,
    class: String,
    item_level: i32,
}

#[derive(Clone, Debug, Serialize)]
struct Activity {
    id: i32,
    name: String,
    difficulty: String,
    completed: bool,
    available: bool,
}

#[derive(Debug, Serialize)]
struct CompleteChar {
    id: i32,
    name: String,
    class: String,
    item_level: i32,
    activities: Vec<Activity>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ActivityUpdate {
    character_id: i32,
    activity_id: i32,
    completed: bool,
}

#[derive(Serialize)]
struct RaidAvailable {
    id: i32,
    completed: i32,
    available: Option<i32>,
}

#[derive(Deserialize, Debug, Default)]
struct CharUpdate {
    cuid: Vec<i32>,
    cid: Vec<i32>,
    cname: Vec<String>,
    oldcname: Vec<String>,
    cclass_id: Vec<i32>,
    oldcclass_id: Vec<i32>,
    item_level: Vec<i32>,
    olditem_level: Vec<i32>,
}

#[post("/me/edit_chars")]
async fn edit_chars_post(
    session: Session,
    pool: web::Data<MySqlPool>,
    form: web::Form<Vec<(String, String)>>
) -> impl Responder {
    let mut update = CharUpdate::default();

    for (k, v) in &form.into_inner() {

        if let Err(_) = match k.as_str() {
            "cuid[]" => { if let Ok(v) = v.parse(){ update.cuid.push(v); Ok(()) } else { Err(()) }},
            "cid[]" => { if let Ok(v) = v.parse(){ update.cid.push(v); Ok(()) } else { Err(()) }},
            "cname[]" => { update.cname.push(v.clone()); Ok(()) },
            "oldcname[]" => { update.oldcname.push(v.clone()); Ok(()) },
            "cclass_id[]" => { if let Ok(v) = v.parse(){ update.cclass_id.push(v); Ok(()) } else { Err(()) }},
            "oldcclass_id[]" => { if let Ok(v) = v.parse(){ update.oldcclass_id.push(v); Ok(()) } else { Err(()) }},
            "item_level[]" => { if let Ok(v) = v.parse(){ update.item_level.push(v); Ok(()) } else { Err(()) }},
            "olditem_level[]" => { if let Ok(v) = v.parse(){ update.olditem_level.push(v); Ok(()) } else { Err(()) }},
            e => { error!("Invalid data in post request: {}", e); Err(()) },
        } {
            return HttpResponse::BadRequest().body("Could not parse data");
        }
    }

    let mut trans = match pool.get_ref().begin().await {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to db"),
    };

    for (cuid, cid, cname, oldcname, cclass_id, oldcclass_id, item_level, olditem_level) in
            izip!(&update.cuid, &update.cid, &update.cname, &update.oldcname, &update.cclass_id, &update.oldcclass_id, &update.item_level, &update.olditem_level) {
        
        if cname != oldcname || cclass_id != oldcclass_id || item_level != olditem_level {
            match sqlx::query!("
                UPDATE characters
                SET name = ?, class_id = ?, item_level = ?
                WHERE id = ? AND user_id = ? AND ? = ?",
                cname, cclass_id, item_level, cid, cuid, cuid, &session.get::<i32>("id").unwrap())
            .execute(&mut trans).await {
                Ok(_) => (),
                Err(e) => error!("{:?}", e),
            }
        }
    }

    if let Err(e) = trans.commit().await {
        error!("{}", e);
        return HttpResponse::InternalServerError().body("Failed to update characters. Please try again later");
    }

    return HttpResponse::SeeOther().insert_header((LOCATION, "chars")).finish();
}

#[get("/me/edit_chars")]
async fn edit_chars(
    tera: web::Data<Tera>,
    session: Session,
    pool: web::Data<MySqlPool>,
) -> impl Responder {
    let mut trans = match pool.get_ref().begin().await {
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
    con.insert("classes", &classes);

    let chars = sqlx::query_as!(
        Character,
        "SELECT * FROM characters WHERE user_id = ? ORDER BY item_level DESC",
        &session.get::<i32>("id").unwrap())
        .fetch_all(&mut trans)
        .await
        .unwrap();

    con.insert("chars", &chars);

    //Ignore errors
    trans.commit().await.ok();

    return HttpResponse::Ok().body(
        tera.render("edit_characters.html", &con).unwrap()
    );
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

    //Ignore errors
    trans.commit().await.ok();

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
        "SELECT ch.id, ch.name as name, cl.name as class, ch.item_level 
        FROM characters ch 
        JOIN classes cl ON ch.class_id = cl.id
        WHERE ch.user_id = ?
        ORDER BY ch.item_level DESC",
        id
    )
    .fetch_all(&mut trans).await;

    let name = sqlx::query!(
        "SELECT username FROM users WHERE id = ?",
        id
    ).fetch_one(&mut trans).await;

    let activities: Vec<Activity> = sqlx::query!(
        "SELECT id, name, difficulty FROM raids ORDER BY required_item_level"
        ).fetch_all(&mut trans)
        .await.unwrap().iter().map(|e| {
            Activity{id: e.id, name: e.name.clone(), difficulty: e.difficulty.clone(), completed: false, available: true}
        }).collect();

    let charc = CharContext {
        activities: activities.clone(),
        name: name.unwrap().username,
        chars: match chars {
            Ok(c) => {
                let mut res: Vec<CompleteChar> = Vec::new();
                for e in c {
                    let activities = sqlx::query!(
                        "SELECT 
                            r.id, r.name, r.difficulty,
                            IF(ur.character_id IS NOT NULL, 1, 0) AS completed,
                            IF(
                                ((r.three_weekly = 1 AND
                                (
                                    SELECT COUNT(*) 
                                    FROM user_raids AS ur2
                                    INNER JOIN raids AS r2 ON ur2.raid_id = r2.id
                                    WHERE ur2.character_id = ? AND r2.three_weekly = 1
                                ) >= 3) OR
                                EXISTS (
                                    SELECT 1 
                                    FROM user_raids AS ur3
                                    INNER JOIN raids AS r3 ON ur3.raid_id = r3.id
                                    WHERE ur3.character_id = ? AND r3.name = r.name
                                ))
                                OR (
                                    EXISTS(
                                        SELECT raid FROM raid_prerequisites WHERE raid NOT IN (
                                            SELECT raid FROM user_raids ur5 JOIN raid_prerequisites rp ON ur5.raid_id = rp.requires WHERE ur5.character_id = ?
                                        ) AND raid = r.id
                                    )
                                ) 
                                OR (
                                    EXISTS(
                                        SELECT *
                                        FROM characters
                                        WHERE id = ? AND item_level < r.required_item_level
                                    )
                                ), 0, 1
                            ) AS available
                        FROM
                            raids AS r
                        LEFT JOIN 
                            user_raids AS ur ON r.id = ur.raid_id AND ur.character_id = ? ORDER BY r.required_item_level;",
                        e.id,
                        e.id,
                        e.id,
                        e.id,
                        e.id,
                    ).fetch_all(&mut trans).await.unwrap();

                    res.push(CompleteChar{ 
                        id: e.id, 
                        name: e.name.clone(), 
                        class: e.class.clone(), 
                        item_level: e.item_level, 
                        activities: activities.iter().map(|f| {
                            Activity{ id: f.id, name: f.name.clone(), difficulty: f.difficulty.clone(), completed: f.completed == 1, available: f.available == Some(1) }
                        }).collect()
                    });
                }
                res
            },
            Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    }};

    if let Err(_) = trans.commit().await {
        error!("Database error");
        return HttpResponse::InternalServerError().body("Database error");
    }

    let html_str = tera.render("characters.html", &Context::from_serialize(&charc).unwrap()).unwrap();

    return HttpResponse::Ok()
        .body(html_str);
}

#[post("/me/update_activity")]
async fn update_activity(
    session: Session,
    pool: web::Data<MySqlPool>,
    update: web::Form<ActivityUpdate>,
) -> impl Responder {
    let id: i32 = session.get("id").unwrap().unwrap();

    let trans = pool.get_ref().begin().await;

    let mut trans = match trans {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to db"),
    };

    let Ok(amount) = sqlx::query!("SELECT COUNT(*) AS count FROM characters WHERE user_id = ? AND id = ?",
        id,
        update.character_id)
        .fetch_one(&mut trans).await else {
            return HttpResponse::Forbidden().finish();
        };

    if amount.count == 0 {
        return HttpResponse::Forbidden().finish();
    }


    match update.completed {
        true => {
            if let Err(e) = sqlx::query!("INSERT INTO user_raids VALUES (?, ?, ?)",
                id,
                update.character_id,
                update.activity_id,
            ).execute(&mut trans)
            .await {
                error!("{:?}", e);
                return HttpResponse::BadRequest().finish();
            }
        },
        false => {
            if let Err(_) = sqlx::query!("DELETE FROM user_raids WHERE user_id = ? AND character_id = ? AND raid_id = ?",
                id,
                update.character_id,
                update.activity_id,
            ).execute(&mut trans).await {
                return HttpResponse::InternalServerError().finish();
            }
        },
    }

    let Ok(res) = sqlx::query_as!(
        RaidAvailable,
        "SELECT 
            r.id,
            IF(ur.character_id IS NOT NULL, 1, 0) AS completed,
            IF(
                ((r.three_weekly = 1 AND
                (
                    SELECT COUNT(*) 
                    FROM user_raids AS ur2
                    INNER JOIN raids AS r2 ON ur2.raid_id = r2.id
                    WHERE ur2.character_id = ? AND r2.three_weekly = 1
                ) >= 3) OR
                EXISTS (
                    SELECT 1 
                    FROM user_raids AS ur3
                    INNER JOIN raids AS r3 ON ur3.raid_id = r3.id
                    WHERE ur3.character_id = ? AND r3.name = r.name
                ))
                OR (
                    EXISTS(
                        SELECT raid FROM raid_prerequisites WHERE raid NOT IN (
                            SELECT raid FROM user_raids ur5 JOIN raid_prerequisites rp ON ur5.raid_id = rp.requires WHERE ur5.character_id = ?
                        ) AND raid = r.id
                    )
                ) OR (
                    EXISTS(
                        SELECT *
                        FROM characters
                        WHERE id = ? AND item_level < r.required_item_level
                    )
                ), 0, 1
            ) AS available
        FROM
            raids AS r
        LEFT JOIN 
            user_raids AS ur ON r.id = ur.raid_id AND ur.character_id = ? ORDER BY r.required_item_level;",
        update.character_id,
        update.character_id,
        update.character_id,
        update.character_id,
        update.character_id,
    )
        .fetch_all(&mut trans)
        .await
    else {
        panic!("asdf");
    };

    if let Err(_) = trans.commit().await {
        return HttpResponse::InternalServerError().body("Failed to update db");
    }

    return HttpResponse::Ok().json(res);
}
