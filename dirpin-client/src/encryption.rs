use crate::settings::Settings;
use base64::prelude::{Engine, BASE64_STANDARD};
use crypto_secretbox::aead::{AeadCore, AeadInPlace, Nonce, OsRng};
use crypto_secretbox::{Key, KeyInit, XSalsa20Poly1305};
use eyre::{bail, ensure, eyre, Context, Report, Result};
use fs_err as fs;
use std::convert::TryInto;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptedItem {
    pub ciphertext: Vec<u8>,
    pub key: Vec<u8>,
    pub key_nonce: Nonce<XSalsa20Poly1305>,
    pub nonce: Nonce<XSalsa20Poly1305>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptedItemEncoded {
    pub ciphertext: String,
    pub key: String,
    pub key_nonce: String,
    pub nonce: String,
}

impl From<EncryptedItem> for EncryptedItemEncoded {
    fn from(item: EncryptedItem) -> Self {
        Self {
            ciphertext: BASE64_STANDARD.encode(item.ciphertext),
            key: BASE64_STANDARD.encode(item.key),
            key_nonce: BASE64_STANDARD.encode(item.key_nonce),
            nonce: BASE64_STANDARD.encode(item.nonce),
        }
    }
}

impl TryFrom<EncryptedItemEncoded> for EncryptedItem {
    type Error = Report;

    fn try_from(item: EncryptedItemEncoded) -> Result<Self> {
        let key_nonce = BASE64_STANDARD
            .decode(item.key_nonce)
            .wrap_err("Invalid size for key nonce")?;
        let key_nonce = Nonce::<XSalsa20Poly1305>::clone_from_slice(&key_nonce);

        let nonce: [u8; 24] = BASE64_STANDARD
            .decode(item.nonce)?
            .try_into()
            .map_err(|_| eyre!("Invalid size for nonce"))?;
        let nonce = Nonce::<XSalsa20Poly1305>::clone_from_slice(&nonce);

        let value = Self {
            ciphertext: BASE64_STANDARD.decode(item.ciphertext)?,
            key: BASE64_STANDARD.decode(item.key)?,
            key_nonce,
            nonce,
        };

        Ok(value)
    }
}

impl EncryptedItem {
    pub fn to_json_base64(self) -> Result<String> {
        let value = serde_json::to_string(&EncryptedItemEncoded::from(self))
            .wrap_err("failed to serialize")?;

        Ok(value)
    }

    pub fn from_json_base64(value: &str) -> Result<Self> {
        let value: EncryptedItemEncoded =
            serde_json::from_str(value).wrap_err("failed to deserialize")?;
        let value = EncryptedItem::try_from(value).wrap_err("failed to decript base64")?;

        Ok(value)
    }
}

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

pub fn read_key<P: AsRef<Path>>(path: P) -> Result<Key> {
    let key = fs_err::read_to_string(path).wrap_err("Failed to read key file")?;
    decode_key(key)
}

pub fn load_key(settings: &Settings) -> Result<Key> {
    let path = settings.key_path.as_str();

    let key = if PathBuf::from(path).exists() {
        read_key(path)?
    } else {
        create_key(settings)?
    };

    Ok(key)
}

pub fn encode_key(key: &Key) -> Result<String> {
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

pub fn decode_key(key: String) -> Result<Key> {
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

pub trait MsgPackSerializable: Sized {
    fn encode_msgpack(&self) -> Result<Vec<u8>>;
    fn decode_msgpack(input: &[u8]) -> Result<Self>;
}

pub fn rmp_error_report<E: std::fmt::Debug>(err: E) -> eyre::Report {
    eyre!("{err:?}")
}

pub fn encrypt<T: MsgPackSerializable>(entry: &T, key: &Key) -> Result<EncryptedItem> {
    let mut entry_buf = entry.encode_msgpack()?;

    let one_time_key = XSalsa20Poly1305::generate_key(&mut OsRng);
    let one_time_key_nonce = XSalsa20Poly1305::generate_nonce(&mut OsRng);
    XSalsa20Poly1305::new(&one_time_key)
        .encrypt_in_place(&one_time_key_nonce, &[], &mut entry_buf)
        .map_err(|_| eyre!("Failed to encrypt data"))?;

    let mut encrypted_key = one_time_key.to_vec();
    let primary_key_nonce = XSalsa20Poly1305::generate_nonce(&mut OsRng);
    XSalsa20Poly1305::new(key)
        .encrypt_in_place(&primary_key_nonce, &[], &mut encrypted_key)
        .map_err(|_| eyre!("Failed to encrypt key"))?;

    Ok(EncryptedItem {
        ciphertext: entry_buf,
        key: encrypted_key,
        key_nonce: one_time_key_nonce,
        nonce: primary_key_nonce,
    })
}

pub fn decrypt<T: MsgPackSerializable>(encrypted_data: EncryptedItem, key: &Key) -> Result<T> {
    let mut one_time_key = encrypted_data.key;
    XSalsa20Poly1305::new(&key)
        .decrypt_in_place(&encrypted_data.nonce, &[], &mut one_time_key)
        .map_err(|_| eyre!("Failed to decrypt data"))?;
    let one_time_key = Key::from_slice(&one_time_key);

    let mut entry = encrypted_data.ciphertext;
    XSalsa20Poly1305::new(&one_time_key)
        .decrypt_in_place(&encrypted_data.key_nonce, &[], &mut entry)
        .map_err(|_| eyre!("Failed to decrypt data"))?;

    let entry = T::decode_msgpack(&entry)?;

    Ok(entry)
}
