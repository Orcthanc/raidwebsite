use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use argon2::password_hash::{SaltString, errors::Error};
use argon2::password_hash::rand_core::OsRng;

#[derive(Clone)]
pub struct CookieSessionSecret{
    pub secret: secrecy::Secret<String>
}

pub fn argon2_hash_text(text: &str) -> Result<String, Error> {
    let password = text.as_bytes();
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password, &salt)?.to_string())
}

pub fn argon2_verify_password(password: &str, hash: &str) -> Result<(), Error>{
    let hash = PasswordHash::new(hash)?;
    Argon2::default().verify_password(password.as_bytes(), &hash)
}
