use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    #[serde(default)]
    pub jti: String,
    pub sub: String, // username
    pub user_id: i64,
    pub is_admin: bool,
    pub exp: usize, // expiry timestamp
    pub iat: usize, // issued at
}

pub fn create_token(
    username: &str,
    user_id: i64,
    is_admin: bool,
    secret: &str,
    expiry_hours: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock before Unix epoch")
        .as_secs() as usize;
    let claims = Claims {
        jti: uuid::Uuid::new_v4().to_string(),
        sub: username.to_string(),
        user_id,
        is_admin,
        exp: now + (expiry_hours * 3600) as usize,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
