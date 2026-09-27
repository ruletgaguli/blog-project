use std::sync::Arc;

use actix_web::{dev::ServiceRequest, error::ErrorUnauthorized, web, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use crate::application::auth_service::AuthService;
use crate::infrastructure::jwt::JwtService;

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub user_id: i64,
}

pub async fn jwt_validator(
    request: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let Some(jwt) = request.app_data::<web::Data<JwtService>>() else {
        return Err((ErrorUnauthorized("JWT service unavailable"), request));
    };
    match jwt.verify_token(credentials.token()) {
        Ok(claims) => {
            let Some(auth) = request.app_data::<web::Data<Arc<AuthService>>>() else {
                return Err((ErrorUnauthorized("auth service unavailable"), request));
            };
            let user = match auth.get_user(claims.user_id).await {
                Ok(user) if user.username == claims.username => user,
                _ => return Err((ErrorUnauthorized("invalid token"), request)),
            };
            request
                .extensions_mut()
                .insert(AuthenticatedUser { user_id: user.id });
            Ok(request)
        }
        Err(_) => Err((ErrorUnauthorized("invalid token"), request)),
    }
}
