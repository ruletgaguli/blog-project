use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    pub exp: usize,
}

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn generate_token(
        &self,
        user_id: i64,
        username: &str,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            user_id,
            username: username.to_string(),
            exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
        };
        encode(&Header::default(), &claims, &self.encoding_key)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        Ok(decode::<Claims>(token, &self.decoding_key, &Validation::default())?.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::JwtService;

    #[test]
    fn token_round_trip_and_tampering() {
        let service = JwtService::new("0123456789abcdef0123456789abcdef");
        let token = service.generate_token(42, "author").unwrap();
        let claims = service.verify_token(&token).unwrap();
        assert_eq!(claims.user_id, 42);
        assert_eq!(claims.username, "author");

        let other_key = JwtService::new("abcdef0123456789abcdef0123456789");
        assert!(other_key.verify_token(&token).is_err());
    }
}
