use sqlx::PgPool;

use crate::domain::{error::BlogError, post::Post};

const POST_COLUMNS: &str = "id, title, content, author_id, created_at, updated_at";

#[derive(Clone)]
pub struct PostRepository {
    pool: PgPool,
}

impl PostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, post: Post) -> Result<Post, BlogError> {
        let query = format!("INSERT INTO posts (title, content, author_id) VALUES ($1, $2, $3) RETURNING {POST_COLUMNS}");
        Ok(sqlx::query_as::<_, Post>(&query)
            .bind(post.title)
            .bind(post.content)
            .bind(post.author_id)
            .fetch_one(&self.pool)
            .await?)
    }

    pub async fn get(&self, id: i64) -> Result<Option<Post>, BlogError> {
        let query = format!("SELECT {POST_COLUMNS} FROM posts WHERE id = $1");
        Ok(sqlx::query_as::<_, Post>(&query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    pub async fn update(
        &self,
        id: i64,
        author_id: i64,
        title: &str,
        content: &str,
    ) -> Result<Option<Post>, BlogError> {
        let query = format!("UPDATE posts SET title = $1, content = $2, updated_at = NOW() WHERE id = $3 AND author_id = $4 RETURNING {POST_COLUMNS}");
        Ok(sqlx::query_as::<_, Post>(&query)
            .bind(title)
            .bind(content)
            .bind(id)
            .bind(author_id)
            .fetch_optional(&self.pool)
            .await?)
    }

    pub async fn delete(&self, id: i64, author_id: i64) -> Result<bool, BlogError> {
        let result = sqlx::query("DELETE FROM posts WHERE id = $1 AND author_id = $2")
            .bind(id)
            .bind(author_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<(Vec<Post>, i64), BlogError> {
        let query = format!(
            "SELECT {POST_COLUMNS} FROM posts ORDER BY created_at DESC, id DESC LIMIT $1 OFFSET $2"
        );
        let posts = sqlx::query_as::<_, Post>(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
            .fetch_one(&self.pool)
            .await?;
        Ok((posts, total))
    }
}
