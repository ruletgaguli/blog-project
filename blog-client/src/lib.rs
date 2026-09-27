pub mod error;
pub mod grpc_client;
pub mod http_client;

use serde::{Deserialize, Serialize};
use std::time::Duration;

use error::BlogClientError;

pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub enum Transport {
    Http(String),
    Grpc(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostList {
    pub posts: Vec<Post>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub struct BlogClient {
    pub transport: Transport,
    http_client: Option<http_client::HttpBlogClient>,
    grpc_client: Option<grpc_client::GrpcBlogClient>,
    token: Option<String>,
}

impl BlogClient {
    pub async fn new(transport: Transport) -> Result<Self, BlogClientError> {
        let (http_client, grpc_client) = match &transport {
            Transport::Http(url) => (Some(http_client::HttpBlogClient::new(url)?), None),
            Transport::Grpc(url) => (None, Some(grpc_client::GrpcBlogClient::connect(url).await?)),
        };
        Ok(Self {
            transport,
            http_client,
            grpc_client,
            token: None,
        })
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }
    pub fn get_token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let response = if let Some(http) = &self.http_client {
            http.register(username, email, password).await?
        } else if let Some(grpc) = &mut self.grpc_client {
            grpc.register(username, email, password).await?
        } else {
            return Err(BlogClientError::InvalidRequest(
                "transport unavailable".into(),
            ));
        };
        self.set_token(response.token.clone());
        Ok(response)
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let response = if let Some(http) = &self.http_client {
            http.login(username, password).await?
        } else if let Some(grpc) = &mut self.grpc_client {
            grpc.login(username, password).await?
        } else {
            return Err(BlogClientError::InvalidRequest(
                "transport unavailable".into(),
            ));
        };
        self.set_token(response.token.clone());
        Ok(response)
    }

    pub async fn create_post(
        &mut self,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let token = self
            .get_token()
            .ok_or(BlogClientError::Unauthorized)?
            .to_string();
        if let Some(http) = &self.http_client {
            http.create_post(&token, title, content).await
        } else if let Some(grpc) = &mut self.grpc_client {
            grpc.create_post(&token, title, content).await
        } else {
            Err(BlogClientError::InvalidRequest(
                "transport unavailable".into(),
            ))
        }
    }

    pub async fn get_post(&mut self, id: i64) -> Result<Post, BlogClientError> {
        if let Some(http) = &self.http_client {
            http.get_post(id).await
        } else if let Some(grpc) = &mut self.grpc_client {
            grpc.get_post(id).await
        } else {
            Err(BlogClientError::InvalidRequest(
                "transport unavailable".into(),
            ))
        }
    }

    pub async fn update_post(
        &mut self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let token = self
            .get_token()
            .ok_or(BlogClientError::Unauthorized)?
            .to_string();
        if let Some(http) = &self.http_client {
            http.update_post(&token, id, title, content).await
        } else if let Some(grpc) = &mut self.grpc_client {
            grpc.update_post(&token, id, title, content).await
        } else {
            Err(BlogClientError::InvalidRequest(
                "transport unavailable".into(),
            ))
        }
    }

    pub async fn delete_post(&mut self, id: i64) -> Result<(), BlogClientError> {
        let token = self
            .get_token()
            .ok_or(BlogClientError::Unauthorized)?
            .to_string();
        if let Some(http) = &self.http_client {
            http.delete_post(&token, id).await
        } else if let Some(grpc) = &mut self.grpc_client {
            grpc.delete_post(&token, id).await
        } else {
            Err(BlogClientError::InvalidRequest(
                "transport unavailable".into(),
            ))
        }
    }

    pub async fn list_posts(
        &mut self,
        limit: i64,
        offset: i64,
    ) -> Result<PostList, BlogClientError> {
        if let Some(http) = &self.http_client {
            http.list_posts(limit, offset).await
        } else if let Some(grpc) = &mut self.grpc_client {
            grpc.list_posts(limit, offset).await
        } else {
            Err(BlogClientError::InvalidRequest(
                "transport unavailable".into(),
            ))
        }
    }
}
