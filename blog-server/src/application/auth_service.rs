use std::sync::Arc;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;

use crate::{
    data::user_repository::UserRepository,
    domain::{
        error::BlogError,
        user::{AuthResponse, LoginRequest, RegisterRequest, User},
    },
    infrastructure::jwt::JwtService,
};

pub struct AuthService {
    users: Arc<UserRepository>,
    jwt: Arc<JwtService>,
}

impl AuthService {
    pub fn new(users: Arc<UserRepository>, jwt: Arc<JwtService>) -> Self {
        Self { users, jwt }
    }

    pub async fn get_user(&self, id: i64) -> Result<User, BlogError> {
        self.users
            .find_by_id(id)
            .await?
            .ok_or(BlogError::UserNotFound)
    }

    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, BlogError> {
        let username = request.username.trim();
        let email = request.email.trim();
        if username.is_empty() || email.is_empty() || request.password.is_empty() {
            return Err(BlogError::InvalidRequest(
                "username, email and password are required".into(),
            ));
        }
        if username.len() > 100 || email.len() > 255 || request.password.len() < 8 {
            return Err(BlogError::InvalidRequest(
                "username or email is too long, or password is shorter than 8 characters".into(),
            ));
        }
        let password = request.password;
        let hash = tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
        })
        .await
        .map_err(|error| {
            tracing::error!(%error, "password hashing task failed");
            BlogError::Internal
        })?
        .map_err(|error| {
            tracing::error!(%error, "password hashing failed");
            BlogError::Internal
        })?;
        let user = self.users.create(username, email, &hash).await?;
        let token = self
            .jwt
            .generate_token(user.id, &user.username)
            .map_err(|error| {
                tracing::error!(%error, "token generation failed");
                BlogError::Internal
            })?;
        Ok(AuthResponse { token, user })
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, BlogError> {
        let user = self
            .users
            .find_by_username(&request.username)
            .await?
            .ok_or(BlogError::InvalidCredentials)?;
        let hash = user.password_hash.clone();
        let password = request.password;
        let valid = tokio::task::spawn_blocking(move || {
            PasswordHash::new(&hash).ok().is_some_and(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
        })
        .await
        .map_err(|error| {
            tracing::error!(%error, "password verification task failed");
            BlogError::Internal
        })?;
        if !valid {
            return Err(BlogError::InvalidCredentials);
        }
        let token = self
            .jwt
            .generate_token(user.id, &user.username)
            .map_err(|error| {
                tracing::error!(%error, "token generation failed");
                BlogError::Internal
            })?;
        Ok(AuthResponse { token, user })
    }
}
