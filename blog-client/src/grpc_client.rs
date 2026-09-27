use tonic::{metadata::MetadataValue, transport::Channel, Request};

use crate::{error::BlogClientError, AuthResponse, Post, PostList, User};

pub mod proto {
    tonic::include_proto!("blog");
}

pub struct GrpcBlogClient {
    client: proto::blog_service_client::BlogServiceClient<Channel>,
}

impl GrpcBlogClient {
    pub async fn connect(url: &str) -> Result<Self, BlogClientError> {
        Ok(Self {
            client: proto::blog_service_client::BlogServiceClient::connect(url.to_string()).await?,
        })
    }

    fn authenticated<T>(body: T, token: &str) -> Result<Request<T>, BlogClientError> {
        let mut request = Request::new(body);
        let header = MetadataValue::try_from(format!("Bearer {token}"))
            .map_err(|error| BlogClientError::InvalidRequest(error.to_string()))?;
        request.metadata_mut().insert("authorization", header);
        Ok(request)
    }

    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let result = self
            .client
            .register(proto::RegisterRequest {
                username: username.into(),
                email: email.into(),
                password: password.into(),
            })
            .await?
            .into_inner();
        auth_response(result)
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let result = self
            .client
            .login(proto::LoginRequest {
                username: username.into(),
                password: password.into(),
            })
            .await?
            .into_inner();
        auth_response(result)
    }

    pub async fn create_post(
        &mut self,
        token: &str,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let request = Self::authenticated(
            proto::CreatePostRequest {
                title: title.into(),
                content: content.into(),
            },
            token,
        )?;
        post_response(self.client.create_post(request).await?.into_inner())
    }

    pub async fn get_post(&mut self, id: i64) -> Result<Post, BlogClientError> {
        post_response(
            self.client
                .get_post(proto::GetPostRequest { id })
                .await?
                .into_inner(),
        )
    }

    pub async fn update_post(
        &mut self,
        token: &str,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let request = Self::authenticated(
            proto::UpdatePostRequest {
                id,
                title: title.into(),
                content: content.into(),
            },
            token,
        )?;
        post_response(self.client.update_post(request).await?.into_inner())
    }

    pub async fn delete_post(&mut self, token: &str, id: i64) -> Result<(), BlogClientError> {
        let request = Self::authenticated(proto::DeletePostRequest { id }, token)?;
        self.client.delete_post(request).await?;
        Ok(())
    }

    pub async fn list_posts(
        &mut self,
        limit: i64,
        offset: i64,
    ) -> Result<PostList, BlogClientError> {
        let response = self
            .client
            .list_posts(proto::ListPostsRequest { limit, offset })
            .await?
            .into_inner();
        Ok(PostList {
            posts: response.posts.into_iter().map(post).collect(),
            total: response.total,
            limit: response.limit,
            offset: response.offset,
        })
    }
}

fn auth_response(response: proto::AuthResponse) -> Result<AuthResponse, BlogClientError> {
    let user = response.user.ok_or_else(|| {
        BlogClientError::InvalidRequest("missing user in authentication response".into())
    })?;
    Ok(AuthResponse {
        token: response.token,
        user: User {
            id: user.id,
            username: user.username,
            email: user.email,
            created_at: user.created_at,
        },
    })
}

fn post_response(response: proto::PostResponse) -> Result<Post, BlogClientError> {
    response
        .post
        .map(post)
        .ok_or_else(|| BlogClientError::InvalidRequest("missing post in response".into()))
}

fn post(post: proto::Post) -> Post {
    Post {
        id: post.id,
        title: post.title,
        content: post.content,
        author_id: post.author_id,
        created_at: post.created_at,
        updated_at: post.updated_at,
    }
}
