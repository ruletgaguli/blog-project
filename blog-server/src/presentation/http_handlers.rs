use std::sync::Arc;

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::{
    application::{auth_service::AuthService, blog_service::BlogService},
    domain::{
        error::BlogError,
        post::{CreatePostRequest, UpdatePostRequest},
        user::{LoginRequest, RegisterRequest},
    },
    presentation::middleware::AuthenticatedUser,
};

pub async fn register(
    auth: web::Data<Arc<AuthService>>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, BlogError> {
    let result = auth.register(body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn login(
    auth: web::Data<Arc<AuthService>>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, BlogError> {
    let result = auth.login(body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

fn authenticated_user(request: &HttpRequest) -> Result<AuthenticatedUser, BlogError> {
    request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .ok_or(BlogError::Unauthorized)
}

pub async fn create_post(
    request: HttpRequest,
    blog: web::Data<Arc<BlogService>>,
    body: web::Json<CreatePostRequest>,
) -> Result<HttpResponse, BlogError> {
    let user = authenticated_user(&request)?;
    let post = blog.create_post(user.user_id, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(post))
}

pub async fn get_post(
    blog: web::Data<Arc<BlogService>>,
    id: web::Path<i64>,
) -> Result<HttpResponse, BlogError> {
    Ok(HttpResponse::Ok().json(blog.get_post(id.into_inner()).await?))
}

pub async fn update_post(
    request: HttpRequest,
    blog: web::Data<Arc<BlogService>>,
    id: web::Path<i64>,
    body: web::Json<UpdatePostRequest>,
) -> Result<HttpResponse, BlogError> {
    let user = authenticated_user(&request)?;
    let post = blog
        .update_post(id.into_inner(), user.user_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(post))
}

pub async fn delete_post(
    request: HttpRequest,
    blog: web::Data<Arc<BlogService>>,
    id: web::Path<i64>,
) -> Result<HttpResponse, BlogError> {
    let user = authenticated_user(&request)?;
    blog.delete_post(id.into_inner(), user.user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_posts(
    blog: web::Data<Arc<BlogService>>,
    query: web::Query<Pagination>,
) -> Result<HttpResponse, BlogError> {
    let list = blog
        .list_posts(query.limit.unwrap_or(10), query.offset.unwrap_or(0))
        .await?;
    Ok(HttpResponse::Ok().json(list))
}
