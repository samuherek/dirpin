use crate::domain::{Entry, EntryKind};
use crate::settings::Settings;
use base64::prelude::{Engine, BASE64_STANDARD};
use crypto_secretbox::aead::{AeadCore, AeadInPlace, Nonce, OsRng};
use crypto_secretbox::{Key, KeyInit, XSalsa20Poly1305};
use eyre::{bail, ensure, eyre, Context, Result};
use fs_err as fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

// type Nonce = GenericArray<u8, U24>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptedEntry {
    pub ciphertext: Vec<u8>,
    pub nonce: Nonce<XSalsa20Poly1305>,
}

#[derive(Debug)]
pub struct DecryptedEntry {}

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

const ENTRY_FIELD_LEN: u32 = 11;

pub fn encode_to_msgpack(entry: &Entry) -> Result<Vec<u8>> {
    use rmp::encode;

    let mut output = Vec::new();
    encode::write_array_len(&mut output, ENTRY_FIELD_LEN)?;

    encode::write_str(&mut output, &entry.id.to_string())?;
    encode::write_str(&mut output, &entry.value)?;
    match &entry.data {
        Some(v) => encode::write_str(&mut output, &v)?,
        None => encode::write_nil(&mut output)?,
    }
    // TODO: fix the kind stuff. Implement &to_str
    encode::write_str(&mut output, &"note")?;
    encode::write_str(&mut output, &entry.hostname)?;
    encode::write_str(&mut output, &entry.cwd)?;
    match &entry.cgd {
        Some(v) => encode::write_str(&mut output, &v)?,
        None => encode::write_nil(&mut output)?,
    }
    encode::write_str(&mut output, &entry.created_at.format(&Rfc3339)?)?;
    encode::write_str(&mut output, &entry.updated_at.format(&Rfc3339)?)?;
    encode::write_u32(&mut output, entry.version)?;
    match entry.deleted_at {
        Some(v) => encode::write_str(&mut output, &v.format(&Rfc3339)?)?,
        None => encode::write_nil(&mut output)?,
    }

    Ok(output)
}

fn rmp_error_report<E: std::fmt::Debug>(err: E) -> eyre::Report {
    eyre!("{err:?}")
}

pub fn decode_from_msgpack(bytes: &[u8]) -> Result<Entry> {
    use rmp::decode;
    use rmp::decode::{Bytes, DecodeStringError};
    use rmp::Marker;

    let mut bytes = Bytes::new(bytes);
    let len = decode::read_array_len(&mut bytes).map_err(rmp_error_report)?;

    if len != ENTRY_FIELD_LEN {
        bail!("incorrectly formed decrypted pin object");
    }

    let bytes = bytes.remaining_slice();
    let (id, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
    let (value, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
    let (data, bytes) = match decode::read_str_from_slice(bytes) {
        Ok((value, bytes)) => (Some(value), bytes),
        Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
            let mut rest = bytes;
            decode::read_nil(&mut rest).map_err(rmp_error_report)?;
            (None, rest)
        }
        Err(e) => return Err(rmp_error_report(e)),
    };
    let (_kind, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
    let (hostname, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
    let (cwd, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
    let (cgd, bytes) = match decode::read_str_from_slice(bytes) {
        Ok((value, bytes)) => (Some(value), bytes),
        Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
            let mut rest = bytes;
            decode::read_nil(&mut rest).map_err(rmp_error_report)?;
            (None, rest)
        }
        Err(e) => return Err(rmp_error_report(e)),
    };
    let (created_at, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
    let (updated_at, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
    let mut bytes = Bytes::new(bytes);
    let version = decode::read_u32(&mut bytes).map_err(rmp_error_report)?;
    let bytes = bytes.remaining_slice();
    let (deleted_at, bytes) = match decode::read_str_from_slice(bytes) {
        Ok((value, bytes)) => (Some(value), bytes),
        Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
            let mut rest = bytes;
            decode::read_nil(&mut rest).map_err(rmp_error_report)?;
            (None, rest)
        }
        Err(e) => return Err(rmp_error_report(e)),
    };

    if !bytes.is_empty() {
        bail!("found more bytes than expected. malformed")
    }

    Ok(Entry {
        id: Uuid::parse_str(id)?,
        value: value.to_owned(),
        data: data.map(|x| x.to_string()),
        // TODO: do the serde_json::from_str() for the kind
        kind: EntryKind::Note,
        hostname: hostname.to_owned(),
        cwd: cwd.to_owned(),
        cgd: cgd.map(|x| x.to_owned()),
        version,
        created_at: OffsetDateTime::parse(created_at, &Rfc3339)?,
        updated_at: OffsetDateTime::parse(updated_at, &Rfc3339)?,
        deleted_at: deleted_at
            .map(|x| OffsetDateTime::parse(x, &Rfc3339))
            .transpose()?,
    })
}

pub fn encrypt(pin: &Entry, key: &Key) -> Result<EncryptedEntry> {
    let mut buf = encode_to_msgpack(pin)?;

    let nonce = XSalsa20Poly1305::generate_nonce(&mut OsRng);
    XSalsa20Poly1305::new(key)
        .encrypt_in_place(&nonce, &[], &mut buf)
        .map_err(|_| eyre!("Failed to encrypt data"))?;

    Ok(EncryptedEntry {
        ciphertext: buf,
        nonce,
    })
}

pub fn decrypt(encrypted_data: EncryptedEntry, key: &Key) -> Result<Entry> {
    let mut buf = encrypted_data.ciphertext;
    XSalsa20Poly1305::new(&key)
        .decrypt_in_place(&encrypted_data.nonce, &[], &mut buf)
        .map_err(|_| eyre!("Failed to decrypt data"))?;

    let pin = decode_from_msgpack(&buf)?;

    Ok(pin)
}
