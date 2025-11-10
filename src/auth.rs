use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClerkClaims {
    pub sub: String,  // Clerk user ID
    pub email: Option<String>,
    pub exp: usize,   // Expiration time
    pub iat: usize,   // Issued at
    pub azp: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub clerk_user_id: String,
    pub email: String,
}

pub struct AuthError(String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, self.0).into_response()
    }
}

/// Clerk JWT validator
pub struct ClerkJwtValidator {
    pub clerk_public_key: String,
}

impl ClerkJwtValidator {
    pub fn from_env() -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let clerk_public_key = std::env::var("CLERK_PUBLIC_KEY")
            .map_err(|_| "CLERK_PUBLIC_KEY environment variable not set")?;

        Ok(Self { clerk_public_key })
    }

    /// Verify Clerk JWT token
    pub fn verify_token(&self, token: &str) -> Result<ClerkClaims, Box<dyn StdError + Send + Sync>> {
        // Decode header to check algorithm
        let header = decode_header(token)?;

        if header.alg != Algorithm::RS256 {
            return Err("Invalid token algorithm, expected RS256".into());
        }

        // Create decoding key from PEM public key
        let decoding_key = DecodingKey::from_rsa_pem(self.clerk_public_key.as_bytes())
            .map_err(|e| format!("Failed to create decoding key: {}", e))?;

        // Set up validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        // Decode and verify token
        let token_data = decode::<ClerkClaims>(token, &decoding_key, &validation)
            .map_err(|e| format!("Token verification failed: {}", e))?;

        Ok(token_data.claims)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError("Missing or invalid Authorization header".to_string()))?;

        // Get Clerk public key from environment
        let validator = ClerkJwtValidator::from_env()
            .map_err(|e| AuthError(format!("Authentication configuration error: {}", e)))?;

        // Verify token
        let claims = validator
            .verify_token(bearer.token())
            .map_err(|e| AuthError(format!("Invalid token: {}", e)))?;

        // Extract email from claims
        let email = claims.email
            .ok_or_else(|| AuthError("Email not found in token".to_string()))?;

        Ok(AuthenticatedUser {
            clerk_user_id: claims.sub,
            email,
        })
    }
}

/// Alternative: Verify Clerk session token via their API
/// This is useful if you don't want to manage public keys
pub async fn verify_clerk_session(session_token: &str) -> Result<ClerkClaims, Box<dyn StdError + Send + Sync>> {
    let clerk_secret_key = std::env::var("CLERK_SECRET_KEY")
        .map_err(|_| "CLERK_SECRET_KEY not set")?;

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.clerk.com/v1/sessions")
        .header("Authorization", format!("Bearer {}", clerk_secret_key))
        .header("Clerk-Session", session_token)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err("Invalid session token".into());
    }

    // Parse session data
    #[derive(Deserialize)]
    struct SessionData {
        user_id: String,
        // Add other fields as needed
    }

    let session: SessionData = response.json().await?;

    // Return simplified claims
    Ok(ClerkClaims {
        sub: session.user_id,
        email: None,  // Need to fetch separately if needed
        exp: 0,
        iat: 0,
        azp: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clerk_validator_creation() {
        // Set test public key
        std::env::set_var("CLERK_PUBLIC_KEY", "-----BEGIN PUBLIC KEY-----\ntest\n-----END PUBLIC KEY-----");

        let result = ClerkJwtValidator::from_env();
        assert!(result.is_ok());
    }
}
