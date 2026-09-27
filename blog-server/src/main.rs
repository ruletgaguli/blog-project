mod application;
mod data;
mod domain;
mod infrastructure;
mod presentation;

use std::{env, net::SocketAddr, sync::Arc};

use actix_cors::Cors;
use actix_web::{guard, web, App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use anyhow::{Context, Result};
use tonic::transport::Server;

use application::{auth_service::AuthService, blog_service::BlogService};
use data::{post_repository::PostRepository, user_repository::UserRepository};
use infrastructure::{
    database::{create_pool, run_migrations},
    jwt::JwtService,
    logging::init_logging,
};
use presentation::{
    grpc_service::{proto::blog_service_server::BlogServiceServer, BlogGrpcService},
    http_handlers,
    middleware::jwt_validator,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    dotenvy::from_filename("blog-server/.env").ok();
    init_logging();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let secret = env::var("JWT_SECRET").context("JWT_SECRET is required")?;
    anyhow::ensure!(
        secret.len() >= 32,
        "JWT_SECRET must be at least 32 characters"
    );
    let http_addr = env::var("HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let grpc_addr: SocketAddr = env::var("GRPC_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:50051".into())
        .parse()?;
    let cors_origin = env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8000".into());

    let pool = create_pool(&database_url)
        .await
        .context("PostgreSQL connection failed")?;
    run_migrations(&pool)
        .await
        .context("database migration failed")?;
    let jwt = Arc::new(JwtService::new(&secret));
    let users = Arc::new(UserRepository::new(pool.clone()));
    let posts = Arc::new(PostRepository::new(pool));
    let auth = Arc::new(AuthService::new(users, Arc::clone(&jwt)));
    let blog = Arc::new(BlogService::new(posts));

    let grpc_service = BlogGrpcService::new(Arc::clone(&auth), Arc::clone(&blog), Arc::clone(&jwt));
    let grpc = Server::builder()
        .add_service(BlogServiceServer::new(grpc_service))
        .serve(grpc_addr);

    let http = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&cors_origin)
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_any_header()
            .max_age(3600);
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(Arc::clone(&auth)))
            .app_data(web::Data::new(Arc::clone(&blog)))
            .app_data(web::Data::new((*jwt).clone()))
            .service(
                web::scope("/api")
                    .guard(guard::fn_guard(|ctx| {
                        ctx.head().method == actix_web::http::Method::GET
                            || ctx.head().uri.path().starts_with("/api/auth/")
                    }))
                    .route("/auth/register", web::post().to(http_handlers::register))
                    .route("/auth/login", web::post().to(http_handlers::login))
                    .route("/posts", web::get().to(http_handlers::list_posts))
                    .route("/posts/{id}", web::get().to(http_handlers::get_post)),
            )
            .service(
                web::scope("/api")
                    .wrap(HttpAuthentication::bearer(jwt_validator))
                    .route("/posts", web::post().to(http_handlers::create_post))
                    .route("/posts/{id}", web::put().to(http_handlers::update_post))
                    .route("/posts/{id}", web::delete().to(http_handlers::delete_post)),
            )
    })
    .bind(&http_addr)
    .with_context(|| format!("HTTP bind failed: {http_addr}"))?
    .run();

    tracing::info!(%http_addr, %grpc_addr, "blog server started");
    tokio::select! {
        result = http => result.context("HTTP server stopped")?,
        result = grpc => result.context("gRPC server stopped")?,
    }
    Ok(())
}
