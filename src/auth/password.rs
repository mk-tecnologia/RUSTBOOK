use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

static PASSWORD_WORKERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

pub async fn hash_async(password: String) -> Result<String, crate::error::ApiError> {
    let permit = PASSWORD_WORKERS
        .try_acquire()
        .map_err(|_| crate::error::ApiError::TooManyRequests)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        hash_password(&password).map_err(|e| crate::error::ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| crate::error::ApiError::Internal(e.to_string()))?
}

pub async fn verify_async(password: String, hash: String) -> Result<bool, crate::error::ApiError> {
    let permit = PASSWORD_WORKERS
        .try_acquire()
        .map_err(|_| crate::error::ApiError::TooManyRequests)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        verify_password(&password, &hash)
    })
    .await
    .map_err(|e| crate::error::ApiError::Internal(e.to_string()))
}
