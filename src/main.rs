use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{get, App, HttpServer, Responder, web::{Data, self}, middleware::Logger, cookie::Key};
use actix_web_lab::middleware::from_fn;
use routes::{register, login, login_form, register_form, logout, reject_unauth_user, base_styles, show_chars, login_style, character_style, add_char, post_add_char};
use routes::UserId;
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
    "Hello, world!"
}

#[get("/")]
async fn index2(user_id: web::ReqData<UserId>) -> impl Responder {
    format!("Hello, {}!", *user_id)
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
                .service(index2)
                .service(logout)
                .service(show_chars)
                .service(add_char)
                .service(post_add_char)
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
