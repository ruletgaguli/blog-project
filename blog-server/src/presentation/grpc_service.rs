use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::{
    application::{auth_service::AuthService, blog_service::BlogService},
    domain::{
        error::BlogError,
        post::{CreatePostRequest, Post, UpdatePostRequest},
        user::{LoginRequest, RegisterRequest, User},
    },
    infrastructure::jwt::JwtService,
};

pub mod proto {
    tonic::include_proto!("blog");
}

pub struct BlogGrpcService {
    auth: Arc<AuthService>,
    blog: Arc<BlogService>,
    jwt: Arc<JwtService>,
}

impl BlogGrpcService {
    pub fn new(auth: Arc<AuthService>, blog: Arc<BlogService>, jwt: Arc<JwtService>) -> Self {
        Self { auth, blog, jwt }
    }

    async fn user_id<T>(&self, request: &Request<T>) -> Result<i64, Status> {
        let header = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| Status::unauthenticated("missing bearer token"))?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("invalid bearer token"))?;
        let claims = self
            .jwt
            .verify_token(token)
            .map_err(|_| Status::unauthenticated("invalid bearer token"))?;
        let user = self
            .auth
            .get_user(claims.user_id)
            .await
            .map_err(|_| Status::unauthenticated("invalid bearer token"))?;
        if user.username != claims.username {
            return Err(Status::unauthenticated("invalid bearer token"));
        }
        Ok(user.id)
    }
}

fn grpc_error(error: BlogError) -> Status {
    match error {
        BlogError::UserNotFound | BlogError::PostNotFound => Status::not_found(error.to_string()),
        BlogError::UserAlreadyExists => Status::already_exists(error.to_string()),
        BlogError::InvalidCredentials | BlogError::Unauthorized => {
            Status::unauthenticated(error.to_string())
        }
        BlogError::Forbidden => Status::permission_denied(error.to_string()),
        BlogError::InvalidRequest(_) => Status::invalid_argument(error.to_string()),
        BlogError::Internal => Status::internal(error.to_string()),
    }
}

fn proto_user(user: User) -> proto::User {
    proto::User {
        id: user.id,
        username: user.username,
        email: user.email,
        created_at: user.created_at.to_rfc3339(),
    }
}

fn proto_post(post: Post) -> proto::Post {
    proto::Post {
        id: post.id,
        title: post.title,
        content: post.content,
        author_id: post.author_id,
        created_at: post.created_at.to_rfc3339(),
        updated_at: post.updated_at.to_rfc3339(),
    }
}

#[tonic::async_trait]
impl proto::blog_service_server::BlogService for BlogGrpcService {
    async fn register(
        &self,
        request: Request<proto::RegisterRequest>,
    ) -> Result<Response<proto::AuthResponse>, Status> {
        let body = request.into_inner();
        let result = self
            .auth
            .register(RegisterRequest {
                username: body.username,
                email: body.email,
                password: body.password,
            })
            .await
            .map_err(grpc_error)?;
        Ok(Response::new(proto::AuthResponse {
            token: result.token,
            user: Some(proto_user(result.user)),
        }))
    }

    async fn login(
        &self,
        request: Request<proto::LoginRequest>,
    ) -> Result<Response<proto::AuthResponse>, Status> {
        let body = request.into_inner();
        let result = self
            .auth
            .login(LoginRequest {
                username: body.username,
                password: body.password,
            })
            .await
            .map_err(grpc_error)?;
        Ok(Response::new(proto::AuthResponse {
            token: result.token,
            user: Some(proto_user(result.user)),
        }))
    }

    async fn create_post(
        &self,
        request: Request<proto::CreatePostRequest>,
    ) -> Result<Response<proto::PostResponse>, Status> {
        let user_id = self.user_id(&request).await?;
        let body = request.into_inner();
        let post = self
            .blog
            .create_post(
                user_id,
                CreatePostRequest {
                    title: body.title,
                    content: body.content,
                },
            )
            .await
            .map_err(grpc_error)?;
        Ok(Response::new(proto::PostResponse {
            post: Some(proto_post(post)),
        }))
    }

    async fn get_post(
        &self,
        request: Request<proto::GetPostRequest>,
    ) -> Result<Response<proto::PostResponse>, Status> {
        let post = self
            .blog
            .get_post(request.into_inner().id)
            .await
            .map_err(grpc_error)?;
        Ok(Response::new(proto::PostResponse {
            post: Some(proto_post(post)),
        }))
    }

    async fn update_post(
        &self,
        request: Request<proto::UpdatePostRequest>,
    ) -> Result<Response<proto::PostResponse>, Status> {
        let user_id = self.user_id(&request).await?;
        let body = request.into_inner();
        let post = self
            .blog
            .update_post(
                body.id,
                user_id,
                UpdatePostRequest {
                    title: body.title,
                    content: body.content,
                },
            )
            .await
            .map_err(grpc_error)?;
        Ok(Response::new(proto::PostResponse {
            post: Some(proto_post(post)),
        }))
    }

    async fn delete_post(
        &self,
        request: Request<proto::DeletePostRequest>,
    ) -> Result<Response<proto::DeletePostResponse>, Status> {
        let user_id = self.user_id(&request).await?;
        self.blog
            .delete_post(request.into_inner().id, user_id)
            .await
            .map_err(grpc_error)?;
        Ok(Response::new(proto::DeletePostResponse {}))
    }

    async fn list_posts(
        &self,
        request: Request<proto::ListPostsRequest>,
    ) -> Result<Response<proto::ListPostsResponse>, Status> {
        let body = request.into_inner();
        let list = self
            .blog
            .list_posts(body.limit, body.offset)
            .await
            .map_err(grpc_error)?;
        Ok(Response::new(proto::ListPostsResponse {
            posts: list.posts.into_iter().map(proto_post).collect(),
            total: list.total,
            limit: list.limit,
            offset: list.offset,
        }))
    }
}

#[cfg(test)]
mod tests {
    use tonic::Code;

    use super::grpc_error;
    use crate::domain::error::BlogError;

    #[test]
    fn business_errors_have_grpc_statuses() {
        assert_eq!(
            grpc_error(BlogError::Forbidden).code(),
            Code::PermissionDenied
        );
        assert_eq!(grpc_error(BlogError::PostNotFound).code(), Code::NotFound);
        assert_eq!(
            grpc_error(BlogError::InvalidRequest("bad".into())).code(),
            Code::InvalidArgument
        );
    }
}
