use reqwest::{Client, Method, Response};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::{status_error, BlogClientError},
    AuthResponse, Post, PostList, CONNECT_TIMEOUT, REQUEST_TIMEOUT,
};

pub struct HttpBlogClient {
    client: Client,
    base_url: String,
}

impl HttpBlogClient {
    pub fn new(url: &str) -> Result<Self, BlogClientError> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .build()?,
            base_url: url.trim_end_matches('/').to_string(),
        })
    }

    async fn read<T: DeserializeOwned>(&self, response: Response) -> Result<T, BlogClientError> {
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            return Err(status_error(status, &body));
        }
        Ok(response.json().await?)
    }

    async fn empty(&self, response: Response) -> Result<(), BlogClientError> {
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            return Err(status_error(status, &body));
        }
        Ok(())
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let response = self
            .client
            .post(self.url("/api/auth/register"))
            .json(
                &serde_json::json!({ "username": username, "email": email, "password": password }),
            )
            .send()
            .await?;
        self.read(response).await
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, BlogClientError> {
        let response = self
            .client
            .post(self.url("/api/auth/login"))
            .json(&serde_json::json!({ "username": username, "password": password }))
            .send()
            .await?;
        self.read(response).await
    }

    async fn send_post<T: Serialize>(
        &self,
        method: Method,
        path: &str,
        token: &str,
        body: T,
    ) -> Result<Post, BlogClientError> {
        let response = self
            .client
            .request(method, self.url(path))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;
        self.read(response).await
    }

    pub async fn create_post(
        &self,
        token: &str,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        self.send_post(
            Method::POST,
            "/api/posts",
            token,
            serde_json::json!({ "title": title, "content": content }),
        )
        .await
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, BlogClientError> {
        let response = self
            .client
            .get(self.url(&format!("/api/posts/{id}")))
            .send()
            .await?;
        self.read(response).await
    }

    pub async fn update_post(
        &self,
        token: &str,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        self.send_post(
            Method::PUT,
            &format!("/api/posts/{id}"),
            token,
            serde_json::json!({ "title": title, "content": content }),
        )
        .await
    }

    pub async fn delete_post(&self, token: &str, id: i64) -> Result<(), BlogClientError> {
        let response = self
            .client
            .delete(self.url(&format!("/api/posts/{id}")))
            .bearer_auth(token)
            .send()
            .await?;
        self.empty(response).await
    }

    pub async fn list_posts(&self, limit: i64, offset: i64) -> Result<PostList, BlogClientError> {
        let response = self
            .client
            .get(self.url("/api/posts"))
            .query(&[("limit", limit), ("offset", offset)])
            .send()
            .await?;
        self.read(response).await
    }
}
