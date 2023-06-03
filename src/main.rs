use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{get, App, HttpServer, Responder, web::{Data, self}, middleware::Logger, cookie::Key, HttpResponse, http::header::LOCATION};
use actix_web_lab::middleware::{from_fn, map_response};
use actix_files::Files;
use openssl::ssl::{SslAcceptor, SslMethod, SslFiletype};
use routes::*;
use crypto::CookieSessionSecret;
use sqlx::mysql::MySqlPoolOptions;
use secrecy::{ExposeSecret, Secret};
use env_logger;
use tera::Tera;

mod data;
mod routes;
mod crypto;

#[get("/")]
async fn index() -> impl Responder {
    return HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().expect("Unable to load environment variables from .env file");

    let db_url = std::env::var("DATABASE_URL").expect("Unable to read DATABASE_URL env var");
    let cookie_secret = CookieSessionSecret{
        secret: Secret::new(std::env::var("COOKIE_SECRET").expect("Unable to read COOKIE_SECRET env var"))
    };

    let tera = Tera::new("templates/**/*.html").unwrap();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Initialize the database connection pool
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Could not connect to database");

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    builder.set_private_key_file("key.pem", SslFiletype::PEM).unwrap();

    builder.set_certificate_chain_file("cert.pem").unwrap();

    // Start the HTTP server
    HttpServer::new(move || {
        App::new()
            // Pass the database connection pool to each handler using Actix Web's data extractor
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(tera.clone()))
            .wrap(SessionMiddleware::new(
                    CookieSessionStore::default(),
                    Key::from(cookie_secret.secret.expose_secret().as_bytes())))
            .wrap(Logger::new("%a: %r, %s"))
            .service(Files::new("/static", "./static").show_files_listing())
            .service(base_styles)
            .service(login_style)
            .service(character_style)
            .service(index)
            .service(register)
            .service(register_form)
            .service(login)
            .service(login_form)
            .service(web::scope("/auth")
                .wrap(from_fn(reject_unauth_user))
                .wrap(map_response(add_private_header))
                .service(logout)
                .service(show_chars)
                .service(add_char)
                .service(post_add_char)
                .service(update_activity)
                .service(edit_chars)
                .service(edit_chars_post)
                .service(view_groups)
                .service(create_group)
                .service(create_group_post)
                .service(edit_group)
                .service(remove_user)
                .service(invite_group)
                .service(invites)
                .service(accept_invite)
                .service(decline_invite)
                .service(view_group)
            )
    })
    .bind_openssl("0.0.0.0:8443", builder)?
    .run()
    .await
}
