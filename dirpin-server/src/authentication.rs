use getrandom::getrandom;
use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};


/// Generate N random bytes, using a cryptographically secure source
pub fn crypto_random_bytes<const N: usize>() -> [u8; N] {
    // rand say they are in principle safe for crypto purposes, but that it is perhaps a better
    // idea to use getrandom for things such as passwords.
    let mut ret = [0u8; N];

    getrandom(&mut ret).expect("Failed to generate random bytes!");

    ret
}

/// Generate N random bytes using a cryptographically secure source, return encoded as a string
pub fn crypto_random_string<const N: usize>() -> String {
    let bytes = crypto_random_bytes::<N>();

    // We only use this to create a random string, and won't be reversing it to find the original
    // data - no padding is OK there. It may be in URLs.
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_password(value: &str) -> eyre::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(value.as_bytes(), &salt)?
    .to_string();
    Ok(password_hash)
}
