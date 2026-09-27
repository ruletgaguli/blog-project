use std::sync::Arc;

use crate::{
    data::post_repository::PostRepository,
    domain::{
        error::BlogError,
        post::{CreatePostRequest, Post, PostList, UpdatePostRequest},
    },
};

pub struct BlogService {
    posts: Arc<PostRepository>,
}

impl BlogService {
    pub fn new(posts: Arc<PostRepository>) -> Self {
        Self { posts }
    }

    pub async fn create_post(
        &self,
        author_id: i64,
        request: CreatePostRequest,
    ) -> Result<Post, BlogError> {
        validate_post(&request.title, &request.content)?;
        self.posts
            .create(Post::new(request.title, request.content, author_id))
            .await
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, BlogError> {
        if id < 1 {
            return Err(BlogError::InvalidRequest("id must be positive".into()));
        }
        self.posts.get(id).await?.ok_or(BlogError::PostNotFound)
    }

    pub async fn update_post(
        &self,
        id: i64,
        author_id: i64,
        request: UpdatePostRequest,
    ) -> Result<Post, BlogError> {
        validate_post(&request.title, &request.content)?;
        self.ensure_author(id, author_id).await?;
        self.posts
            .update(id, author_id, &request.title, &request.content)
            .await?
            .ok_or(BlogError::PostNotFound)
    }

    pub async fn delete_post(&self, id: i64, author_id: i64) -> Result<(), BlogError> {
        self.ensure_author(id, author_id).await?;
        if self.posts.delete(id, author_id).await? {
            Ok(())
        } else {
            Err(BlogError::PostNotFound)
        }
    }

    pub async fn list_posts(&self, limit: i64, offset: i64) -> Result<PostList, BlogError> {
        if !(1..=100).contains(&limit) || offset < 0 {
            return Err(BlogError::InvalidRequest(
                "limit must be 1..100 and offset must be non-negative".into(),
            ));
        }
        let (posts, total) = self.posts.list(limit, offset).await?;
        Ok(PostList {
            posts,
            total,
            limit,
            offset,
        })
    }

    async fn ensure_author(&self, id: i64, author_id: i64) -> Result<(), BlogError> {
        let post = self.get_post(id).await?;
        if post.author_id == author_id {
            Ok(())
        } else {
            Err(BlogError::Forbidden)
        }
    }
}

fn validate_post(title: &str, content: &str) -> Result<(), BlogError> {
    if title.trim().is_empty() || content.trim().is_empty() || title.len() > 255 {
        Err(BlogError::InvalidRequest(
            "title and content are required; title must be at most 255 bytes".into(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_post;
    #[test]
    fn rejects_empty_post_fields() {
        assert!(validate_post(" ", "body").is_err());
        assert!(validate_post("title", " ").is_err());
        assert!(validate_post("title", "body").is_ok());
    }
}
