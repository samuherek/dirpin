use crate::settings::Settings;
use base64::prelude::{Engine, BASE64_STANDARD};
use crypto_secretbox::aead::OsRng;
use crypto_secretbox::{Key, KeyInit, XSalsa20Poly1305};
use eyre::{bail, ensure, eyre, Context, Result};
use fs_err;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn generate_encoded_key() -> Result<(Key, String)> {
    let key = XSalsa20Poly1305::generate_key(&mut OsRng);
    let encoded = encode_key(&key)?;
    Ok((key, encoded))
}

pub fn create_key(settings: &Settings) -> Result<Key> {
    let path = PathBuf::from(settings.key_path.as_str());

    if path.exists() {
        bail!("key already exists. not allowed to overwrite");
    }

    let (key, encoded) = generate_encoded_key()?;
    let mut file = fs::File::create(path)?;
    file.write_all(encoded.as_bytes())?;

    Ok(key)
}

pub fn load_key(settings: &Settings) -> Result<Key> {
    let path = settings.key_path.as_str();

    let key = if PathBuf::from(path).exists() {
        let key = fs_err::read_to_string(path)?;
        decode_key(key)?
    } else {
        create_key(settings)?
    };

    Ok(key)
}

fn encode_key(key: &Key) -> Result<String> {
    let mut buf = vec![];
    rmp::encode::write_array_len(&mut buf, key.len() as u32)
        .context("Failed to encode key to message pack")?;
    for byte in key {
        rmp::encode::write_uint(&mut buf, *byte as u64)
            .context("Failed to encode key to message pack")?;
    }
    let buf = BASE64_STANDARD.encode(buf);
    Ok(buf)
}

fn decode_key(key: String) -> Result<Key> {
    let buf = BASE64_STANDARD
        .decode(key.trim_end())
        .context("Failed to decode key from base64")?;

    let mut buf = rmp::decode::Bytes::new(&buf);
    let len = rmp::decode::read_array_len(&mut buf).map_err(|err| eyre!("{err:?}"))?;
    ensure!(len == 32, "encryption key is not the correct size");

    let mut key = Key::default();
    for v in &mut key {
        *v = rmp::decode::read_int(&mut buf).map_err(|err| eyre!("{err:?}"))?;
    }

    Ok(key)
}
