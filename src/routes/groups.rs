use actix_session::Session;
use actix_web::{get, post, HttpResponse, web, Responder, http::header};
use log::{error, warn};
use sqlx::{MySqlPool, Row};
use tera::{Tera, Context};
use crate::data::Group;
use serde::{Deserialize, Serialize};

#[get("/groups/{id}")]
async fn view_group(
    tera: web::Data<Tera>,
    session: Session,
    pool: web::Data<MySqlPool>,
    gid: web::Path<(i32,)>,
) -> impl Responder {
    let gid = gid.0;
    let session = session.get::<i32>("id").unwrap();
    
    let mut trans = match pool.begin().await {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to database"),
    };

    let gname = match sqlx::query!(
        "SELECT g.name FROM groups g
        JOIN group_members gm
        ON g.id = gm.group_id
        WHERE gm.user_id = ? AND g.id = ?",
        session,
        gid)
    .fetch_optional(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    let gname = match gname {
        None => return HttpResponse::Forbidden().body("You are not a group member"),
        Some(v) => v.name,
    };

    let members = match sqlx::query!(
        "SELECT gm.user_id, u.username FROM group_members gm
        JOIN users u
        ON u.id = gm.user_id
        WHERE gm.group_id = ?",
        gid
    ).fetch_all(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    #[derive(Deserialize, Debug)]
    struct MemberEntry {
        id: i32,
        rname: Option<String>,
        name: Option<String>,
        support: Option<u8>,
        gives_gold: Option<i32>,
    }

    #[derive(Serialize, Debug, Default)]
    struct RenderableRaid {
        name: String,
        dd: Vec<String>,
        dd_nogold: Vec<String>,
        support: Vec<String>,
        support_nogold: Vec<String>,
    }

    #[derive(Serialize, Debug, Default)]
    struct RenderableUser {
        name: String,
        raids: Vec<RenderableRaid>,
    }

    let mut users = Vec::new();

    for m in &members {
        let raids = match sqlx::query!(
            r#"SELECT CONCAT(name, " ", difficulty) AS rname
            FROM raids
            ORDER BY required_item_level"#)
        .fetch_all(&mut trans)
        .await {
            Ok(v) => v,
            Err(e) => {
                error!("{:?}", e);
                return HttpResponse::InternalServerError().body("Database error");
            },
        };

        let ent = match sqlx::query_as!(
            MemberEntry,
            r#"WITH 
                amount AS (
                    SELECT c.id, c.name, 3 - SUM(IFNULL(three_weekly, 0)) AS entries
                    FROM characters c
                    LEFT JOIN user_raids ur
                    ON ur.character_id = c.id
                    LEFT JOIN raids r
                    ON ur.raid_id = r.id
                    WHERE c.user_id = ?
                    GROUP BY c.id
                ),
                r_amount AS (
                    SELECT count(character_id) AS a FROM (
                        SELECT * FROM user_raids
                        WHERE user_id = ?
                        GROUP BY character_id
                    ) AS t
                ),
                available AS (
                    SELECT 
                        r.id,
                        CONCAT(r.name, " ", r.difficulty) AS rname,
                        c.name,
                        cl.support,
                        IF(
                            a.entries <= 0 AND r.three_weekly
                            OR ((SELECT a FROM r_amount) >= 6 AND NOT EXISTS(SELECT * FROM user_raids WHERE character_id = c.id)),
                            0,
                            1) AS gives_gold
                    FROM raids r
                    JOIN characters c
                    JOIN amount a
                    ON a.id = c.id
                    JOIN classes cl
                    ON cl.id = c.class_id
                    WHERE c.user_id = ?
                    AND NOT EXISTS(
                        SELECT * FROM user_raids ur
                        JOIN raids r2
                        ON r2.id = ur.raid_id
                        WHERE r2.name = r.name AND c.id = ur.character_id
                    ) AND r.required_item_level < c.item_level
                    AND (
                        EXISTS (
                            SELECT * FROM raid_prerequisites rp
                            JOIN user_raids ur
                            ON rp.requires = ur.raid_id
                            WHERE ur.character_id = c.id AND r.id = rp.raid
                        ) OR NOT EXISTS (
                            SELECT * FROM raid_prerequisites rp WHERE r.id = rp.raid
                        )
                    )
                )
            SELECT r.id, CONCAT(r.name, " ", r.difficulty) AS rname, a.name, a.support, a.gives_gold
            FROM raids r
            LEFT JOIN available a
            ON r.id = a.id
            ORDER BY r.required_item_level"#,
            m.user_id,
            m.user_id,
            m.user_id,
        )
        .fetch_all(&mut trans)
        .await {
            Ok(v) => v,
            Err(e) => {
                error!("{:?}", e);
                return HttpResponse::InternalServerError().body("Database error");
            },
        };

        let mut last_id = -1;

        let mut uraids = Vec::new();

        for e in ent.iter() {
            //user.entry(e.id).or_insert_with(|| (e.rname.clone(), [Vec::<String>::new(), Vec::<String>::new()])).1[e.gives_gold.unwrap() as usize].push(e.name.clone());
            if last_id != e.id {
                last_id = e.id;
                uraids.push(RenderableRaid {
                    name: e.rname.as_ref().unwrap().clone(),
                    ..Default::default()
                });
            }

            match (e.support, e.gives_gold) {
                (Some(1), Some(1)) => uraids.last_mut().unwrap().support.push(e.name.as_ref().unwrap().clone()),
                (Some(0), Some(1)) => uraids.last_mut().unwrap().dd.push(e.name.as_ref().unwrap().clone()),
                (Some(1), Some(0)) => uraids.last_mut().unwrap().support_nogold.push(e.name.as_ref().unwrap().clone()),
                (Some(0), Some(0)) => uraids.last_mut().unwrap().dd_nogold.push(e.name.as_ref().unwrap().clone()),
                _ => ()
            };
        }


        users.push(
            RenderableUser {
                name: m.username.clone(),
                raids: uraids
            }
        );
    }

    match trans.commit().await {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    let mut con = Context::new();
    con.insert("gname", &gname);
    con.insert("users", &users);

    return HttpResponse::Ok().body(tera.render("view_group.html", &con).unwrap());
}

#[get("/me/groups")]
async fn view_groups(
    tera: web::Data<Tera>,
    session: Session,
    pool: web::Data<MySqlPool>,
) -> impl Responder {
    let id = session.get::<i32>("id").unwrap();

    let mut trans = match pool.begin().await {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to database"),
    };

    let groups = match sqlx::query_as!(
        Group,
        "SELECT g.id, g.name, g.creator_id
        FROM groups g
        JOIN group_members gm
        ON g.id = gm.group_id
        WHERE gm.user_id = ?",
        id
    ).fetch_all(&mut trans).await {
        Ok(v) => v,
        Err(_) => return HttpResponse::InternalServerError().body("Database Error"),
    };

    match trans.commit().await {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    let mut con = Context::new();
    con.insert("user_id", &id);
    con.insert("groups", &groups);

    HttpResponse::Ok().body(tera.render("view_groups.html", &con).unwrap())
}

#[get("/me/invites")]
async fn invites(
    tera: web::Data<Tera>,
    session: Session,
    pool: web::Data<MySqlPool>,
) -> impl Responder {
    let id = session.get::<i32>("id").unwrap();

    let mut trans = match pool.begin().await {
        Ok(t) => t,
        Err(_) => panic!("Failed to connect to database"),
    };

    #[derive(Deserialize, Serialize)]
    struct RenderableInvite {
        id: i32,
        src: String,
        groupn: String,
    }

    let invites = match sqlx::query_as!(
        RenderableInvite,
        "SELECT i.id, u.username AS src, g.name AS groupn
        FROM invites i
        JOIN users u
        ON i.source = u.id
        JOIN groups g
        ON i.group_id = g.id
        WHERE dest = ?",
        id,
    )
    .fetch_all(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    match trans.commit().await {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    let mut con = Context::new();
    con.insert("invites", &invites);

    return HttpResponse::Ok().body(tera.render("invites.html", &con).unwrap());
}

#[get("/me/invites/accept/{id}")]
async fn accept_invite(
    pool: web::Data<MySqlPool>,
    session: Session,
    iid: web::Path<(u32, )>,
) -> impl Responder {
    let id = session.get::<i32>("id").unwrap();
    let iid = iid.0;

    let mut trans = match pool.begin().await {
        Ok(v) => v,
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    let res = match sqlx::query!(
        "SELECT * FROM invites WHERE dest = ? AND id = ?",
        id,
        iid,
    )
    .fetch_optional(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    if res.is_none() {
        return HttpResponse::Forbidden().body("This is not a pending invite that can be accepted");
    }

    match sqlx::query!(
        "INSERT INTO group_members(user_id, group_id) SELECT dest, group_id FROM invites WHERE id = ?;",
        iid,
    )
    .execute(&mut trans)
    .await {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };
    
    match sqlx::query!(
        "DELETE FROM invites WHERE id = ?;",
        iid,
    )
    .execute(&mut trans)
    .await {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    match trans.commit().await {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    return HttpResponse::SeeOther().insert_header((header::LOCATION, "../../invites")).finish();
}

#[get("/me/invites/decline/{id}")]
async fn decline_invite(
    pool: web::Data<MySqlPool>,
    session: Session,
    iid: web::Path<(u32, )>,
) -> impl Responder {
    let id = session.get::<i32>("id").unwrap();
    let iid = iid.0;

    let mut trans = match pool.begin().await {
        Ok(v) => v,
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    let res = match sqlx::query!(
        "SELECT * FROM invites WHERE dest = ? AND id = ?",
        id,
        iid,
    )
    .fetch_optional(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    if res.is_none() {
        return HttpResponse::Forbidden().body("This is not a pending invite that can be accepted");
    }

    match sqlx::query!(
        "DELETE FROM invites WHERE id = ?;",
        iid,
    )
    .execute(&mut trans)
    .await {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    match trans.commit().await {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    return HttpResponse::SeeOther().insert_header((header::LOCATION, "../../invites")).finish();
}

#[get("/me/groups/new")]
async fn create_group(
    tera: web::Data<Tera>,
) -> impl Responder {
    HttpResponse::Ok().body(tera.render("create_group.html", &Context::new()).unwrap())
}

#[derive(Deserialize)]
struct NameForm {
    name: String,
}

#[post("/me/groups/new")]
async fn create_group_post(
    pool: web::Data<MySqlPool>,
    session: Session,
    name: web::Form<NameForm>,
) -> impl Responder {
    let mut trans = match pool.begin().await {
        Ok(v) => v,
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    let gid = match sqlx::query(
        "INSERT INTO groups (name, creator_id) VALUES (?, ?) RETURNING id;"
    )
    .bind(name.name.clone())
    .bind(session.get::<i32>("id").unwrap())
    .fetch_one(&mut trans)
    .await {
        Ok(v) => dbg!(v).get::<i32, _>(0),
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    match sqlx::query!(
        "INSERT INTO group_members VALUES (?, ?);",
        gid,
        session.get::<i32>("id").unwrap(),
    ).execute(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },

    };

    match trans.commit().await {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    HttpResponse::SeeOther().insert_header((header::LOCATION, "/auth/me/groups")).finish()
}

#[post("/me/groups/edit/{id}/invite")]
async fn invite_group(
    pool: web::Data<MySqlPool>,
    session: Session,
    group_id: web::Path<(u32,)>,
    name: web::Form<NameForm>,
) -> impl Responder {
    let session = session.get::<i32>("id").unwrap();

    let group_id = group_id.0;

    let mut trans = match pool.begin().await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    let group = match sqlx::query!(
        "SELECT *
        FROM groups
        WHERE id = ? AND creator_id = ?",
        group_id,
        session,
    ).fetch_optional(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    if group.is_none() {
        return HttpResponse::Forbidden().body("Only the owner can invite people");
    }

    match sqlx::query!(
        "INSERT INTO invites(source, dest, group_id) SELECT ?, id, ? FROM users WHERE username=?",
        session,
        group_id,
        name.name,
    )
    .execute(&mut trans)
    .await {
        Ok(_) => (),
        Err(e) => {
            warn!("{:?}", e);
            return HttpResponse::BadRequest().body(format!("Could not invite {}", name.name));
        },
    }

    match trans.commit().await {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    return HttpResponse::SeeOther().insert_header((header::LOCATION, format!("../{}", group_id))).finish();
}

#[get("/me/groups/edit/{id}")]
async fn edit_group(
    pool: web::Data<MySqlPool>,
    session: Session,
    group_id: web::Path<(u32,)>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let mut trans = match pool.begin().await {
        Ok(v) => v,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to connect to database"),
    };

    let session = session.get::<i32>("id").unwrap();

    let group = match sqlx::query!(
        "SELECT *
        FROM groups
        WHERE id = ? AND creator_id = ?",
        group_id.0,
        session,
    ).fetch_optional(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    if group.is_none() {
        return HttpResponse::Forbidden().body("Only the owner can access the admin informations");
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct RenderableGroupMember {
        id: i32,
        name: String,
    }

    let members = match sqlx::query_as!(
        RenderableGroupMember,
        "SELECT u.id, u.username AS name
        FROM users u
        JOIN group_members g
        ON u.id = g.user_id
        WHERE g.group_id = ?",
        group_id.0,
    )
    .fetch_all(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    let group_name = match sqlx::query!(
        "SELECT name
        FROM groups
        WHERE id = ?",
        group_id.0,
    )
    .fetch_one(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    let mut con = Context::new();
    con.insert("members", &members);
    con.insert("gname", &group_name.name);
    con.insert("group", &group_id.0);

    match trans.commit().await {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    return HttpResponse::Ok().body(tera.render("edit_group.html", &con).unwrap());
}

#[get("/me/groups/remove/{gid}/{uid}")]
async fn remove_user(
    pool: web::Data<MySqlPool>,
    session: Session,
    vals: web::Path<(i32, i32, )>,
) -> impl Responder {
    let (gid, uid) = vals.into_inner();

    let id = session.get::<i32>("id").unwrap();

    let mut trans = match pool.begin().await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    let group = match sqlx::query!(
        "SELECT *
        FROM groups
        WHERE id = ? AND creator_id = ?",
        gid,
        id,
    ).fetch_optional(&mut trans)
    .await {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    };

    if group.is_none() {
        return HttpResponse::Forbidden().body("Only the owner can access the admin informations");
    }

    match sqlx::query!(
        "DELETE FROM group_members
        WHERE group_id = ? AND user_id = ?",
        gid, uid,
    )
    .execute(&mut trans)
    .await {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            return HttpResponse::InternalServerError().body("Database error");
        },
    }
    
    match trans
        .commit()
        .await {
            Ok(_) => (),
            Err(e) => {
                error!("{:?}", e);
                return HttpResponse::InternalServerError().body("Database error");
            },
        }

    return HttpResponse::SeeOther().insert_header((header::LOCATION, format!("../../edit/{}", gid))).body("Not yet implemented");
}
