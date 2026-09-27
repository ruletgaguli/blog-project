use sqlx::PgPool;

use crate::domain::{error::BlogError, user::User};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, username: &str, email: &str, hash: &str) -> Result<User, BlogError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id, username, email, password_hash, created_at",
        )
        .bind(username).bind(email).bind(hash)
        .fetch_one(&self.pool).await
        .map_err(|error| {
            if error.as_database_error().and_then(|db| db.code()).as_deref() == Some("23505") {
                BlogError::UserAlreadyExists
            } else {
                error.into()
            }
        })
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, BlogError> {
        Ok(sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<User>, BlogError> {
        Ok(sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }
}
