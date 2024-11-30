use crate::domain::host::HostId;
use crate::domain::workspace::WorkspaceId;
use crate::encryption::{rmp_error_report, MsgPackSerializable};
use dirpin_common::domain::SyncVersion;
use eyre::{bail, Result};
use rmp::decode::{self, Bytes, DecodeStringError};
use rmp::encode;
use rmp::Marker;
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
/// TODO: Add the ability to have custom types
pub enum EntryKind {
    Note,
    Cmd,
    Todo,
}

impl FromStr for EntryKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "note" => Ok(Self::Note),
            "cmd" => Ok(Self::Cmd),
            "todo" => Ok(Self::Todo),
            _ => Ok(Self::Note),
        }
    }
}

impl EntryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntryKind::Note => "note",
            EntryKind::Cmd => "cmd",
            EntryKind::Todo => "todo",
        }
    }
}

impl std::fmt::Display for EntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct Entry {
    pub id: Uuid,
    pub value: String,
    pub desc: Option<String>,
    pub data: Option<String>,
    pub kind: EntryKind,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
    pub version: SyncVersion,
    pub path: String,
    pub workspace_id: Option<WorkspaceId>,
    pub host_id: HostId,
}

impl Entry {
    const FIELD_LEN: u32 = 11;

    pub fn new(
        value: String,
        path: String,
        workspace_id: Option<WorkspaceId>,
        host_id: HostId,
    ) -> Self {
        let id = Uuid::now_v7();
        let updated_at = OffsetDateTime::now_utc();
        Self {
            id,
            value,
            desc: None,
            data: None,
            kind: EntryKind::Note,
            updated_at,
            deleted_at: None,
            version: SyncVersion::new(),
            path,
            workspace_id,
            host_id,
        }
    }

    pub fn kind(mut self, kind: EntryKind) -> Self {
        self.kind = kind;
        self
    }
}

// TODO: I did it withouth serde for the learning process with message pack.
// Maybe we should move to serde deserialize and serialize as this seems rather
// error prone and ugly :D
impl MsgPackSerializable for Entry {
    fn encode_msgpack(&self) -> Result<Vec<u8>> {
        let mut count = 0;
        let mut output = Vec::new();
        encode::write_array_len(&mut output, Self::FIELD_LEN)?;

        encode::write_str(&mut output, &self.id.to_string())?;
        count += 1;
        encode::write_str(&mut output, &self.value)?;
        count += 1;
        match &self.desc {
            Some(v) => encode::write_str(&mut output, &v)?,
            None => encode::write_nil(&mut output)?,
        }
        count += 1;
        match &self.data {
            Some(v) => encode::write_str(&mut output, &v)?,
            None => encode::write_nil(&mut output)?,
        }
        count += 1;
        encode::write_str(&mut output, &self.path)?;
        count += 1;
        encode::write_str(&mut output, &self.kind.to_string())?;
        count += 1;
        encode::write_str(&mut output, &self.updated_at.format(&Rfc3339)?)?;
        count += 1;
        match self.deleted_at {
            Some(v) => encode::write_str(&mut output, &v.format(&Rfc3339)?)?,
            None => encode::write_nil(&mut output)?,
        }
        count += 1;
        encode::write_u32(&mut output, self.version.inner())?;
        count += 1;
        match &self.workspace_id {
            Some(v) => encode::write_str(&mut output, &v.to_string())?,
            None => encode::write_nil(&mut output)?,
        }
        count += 1;
        encode::write_str(&mut output, &self.host_id.to_string())?;
        count += 1;

        assert_eq!(count, Self::FIELD_LEN);

        Ok(output)
    }

    fn decode_msgpack(input: &[u8]) -> Result<Entry> {
        let mut count = 0;
        let mut bytes = Bytes::new(input);
        let len = decode::read_array_len(&mut bytes).map_err(rmp_error_report)?;

        if len != Self::FIELD_LEN {
            bail!("incorrectly formed decrypted entry object");
        }

        let bytes = bytes.remaining_slice();
        let (id, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;
        let (value, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;
        let (desc, bytes) = match decode::read_str_from_slice(bytes) {
            Ok((value, bytes)) => (Some(value), bytes),
            Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
                let mut rest = bytes;
                decode::read_nil(&mut rest).map_err(rmp_error_report)?;
                (None, rest)
            }
            Err(e) => return Err(rmp_error_report(e)),
        };
        count += 1;
        let (data, bytes) = match decode::read_str_from_slice(bytes) {
            Ok((value, bytes)) => (Some(value), bytes),
            Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
                let mut rest = bytes;
                decode::read_nil(&mut rest).map_err(rmp_error_report)?;
                (None, rest)
            }
            Err(e) => return Err(rmp_error_report(e)),
        };
        count += 1;
        let (path, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;
        let (kind, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;
        let (updated_at, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;
        let (deleted_at, bytes) = match decode::read_str_from_slice(bytes) {
            Ok((value, bytes)) => (Some(value), bytes),
            Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
                let mut rest = bytes;
                decode::read_nil(&mut rest).map_err(rmp_error_report)?;
                (None, rest)
            }
            Err(e) => return Err(rmp_error_report(e)),
        };
        count += 1;
        let mut bytes = Bytes::new(bytes);
        let version = decode::read_u32(&mut bytes).map_err(rmp_error_report)?;
        let bytes = bytes.remaining_slice();
        count += 1;
        let (workspace_id, bytes) = match decode::read_str_from_slice(bytes) {
            Ok((value, bytes)) => (Some(value), bytes),
            Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
                let mut rest = bytes;
                decode::read_nil(&mut rest).map_err(rmp_error_report)?;
                (None, rest)
            }
            Err(e) => return Err(rmp_error_report(e)),
        };
        count += 1;
        let (host_id, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;

        if count != Self::FIELD_LEN {
            bail!("incorrectly encoded message pack bytes.");
        }

        if !bytes.is_empty() {
            bail!("found more bytes than expected. malformed")
        }

        Ok(Entry {
            id: Uuid::parse_str(id)?,
            value: value.to_owned(),
            desc: desc.map(|x| x.to_owned()),
            data: data.map(|x| x.to_owned()),
            kind: EntryKind::from_str(kind).unwrap(),
            path: path.to_owned(),
            updated_at: OffsetDateTime::parse(updated_at, &Rfc3339)?,
            deleted_at: deleted_at
                .map(|x| OffsetDateTime::parse(x, &Rfc3339))
                .transpose()?,
            version: SyncVersion::from(version),
            workspace_id: workspace_id.map(|x| x.parse().ok()).flatten(),
            host_id: HostId::from_str(host_id).unwrap(),
        })
    }
}
