use crate::domain::context::Context;
use crate::domain::host::HostId;
use crate::encryption::{rmp_error_report, MsgPackSerializable};
use dirpin_common::domain::SyncVersion;
use eyre::{bail, Result};
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Hash, Eq, PartialEq)]
pub struct WorkspaceId(Uuid);

impl WorkspaceId {
    pub fn inner(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl FromStr for WorkspaceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspacePath {
    host_id: HostId,
    path: String,
}

impl WorkspacePath {
    pub fn new(host_id: HostId, path: String) -> Self {
        Self { host_id, path }
    }
}

impl TryFrom<&str> for WorkspacePath {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (host, path) = value
            .split_once(':')
            .ok_or("Incorrect workspace path format")?;

        let host_id = HostId::from_str(host).map_err(|_| "Incorrect host format".to_string())?;

        Ok(Self {
            host_id,
            path: path.into(),
        })
    }
}

impl std::fmt::Display for WorkspacePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host_id, self.path)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub git: Option<String>,
    pub paths: Vec<WorkspacePath>,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
    pub version: SyncVersion,
}

impl Workspace {
    const FIELD_LEN: u32 = 7;

    pub fn new(name: String, context: &Context) -> Self {
        // TODO: get the name based on global context and only take out the name of the directory
        Self {
            id: WorkspaceId(Uuid::now_v7()),
            name,
            git: context.git.clone(),
            paths: vec![WorkspacePath::new(
                context.host_id.clone(),
                context.path.clone(),
            )],
            updated_at: OffsetDateTime::now_utc(),
            deleted_at: None,
            version: SyncVersion::new(),
        }
    }
}

// TODO: I did it withouth serde for the learning process with message pack.
// Maybe we should move to serde deserialize and serialize as this seems rather
// error prone and ugly :D
impl MsgPackSerializable for Workspace {
    fn encode_msgpack(&self) -> Result<Vec<u8>> {
        use rmp::encode;

        let mut count = 0;
        let mut output = Vec::new();
        encode::write_array_len(&mut output, Self::FIELD_LEN)?;

        encode::write_str(&mut output, &self.id.to_string())?;
        count += 1;
        encode::write_str(&mut output, &self.name)?;
        count += 1;
        match &self.git {
            Some(v) => encode::write_str(&mut output, &v)?,
            None => encode::write_nil(&mut output)?,
        }
        count += 1;
        encode::write_str(
            &mut output,
            &self
                .paths
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(","),
        )?;
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

        assert_eq!(count, Self::FIELD_LEN);

        Ok(output)
    }

    fn decode_msgpack(input: &[u8]) -> Result<Workspace> {
        use rmp::decode::{self, Bytes, DecodeStringError};
        use rmp::Marker;

        let mut count = 0;
        let mut bytes = Bytes::new(input);
        let len = decode::read_array_len(&mut bytes).map_err(rmp_error_report)?;

        if len != Self::FIELD_LEN {
            bail!("incorrectly formed decrypted entry object");
        }

        let bytes = bytes.remaining_slice();
        let (id, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;
        let (name, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
        count += 1;
        let (git, bytes) = match decode::read_str_from_slice(bytes) {
            Ok((value, bytes)) => (Some(value), bytes),
            Err(DecodeStringError::TypeMismatch(Marker::Null)) => {
                let mut rest = bytes;
                decode::read_nil(&mut rest).map_err(rmp_error_report)?;
                (None, rest)
            }
            Err(e) => return Err(rmp_error_report(e)),
        };
        count += 1;
        let (paths, bytes) = decode::read_str_from_slice(bytes).map_err(rmp_error_report)?;
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

        if count != Self::FIELD_LEN {
            bail!("incorrectly encoded message pack bytes.");
        }

        if !bytes.is_empty() {
            bail!("found more bytes than expected. malformed")
        }

        Ok(Workspace {
            id: WorkspaceId::from_str(id)?,
            name: name.to_owned(),
            git: git.map(|x| x.to_owned()),
            paths: paths
                .split(",")
                .map(|x| WorkspacePath::try_from(x).expect("failed to parse workspace path"))
                .collect(),
            updated_at: OffsetDateTime::parse(updated_at, &Rfc3339)?,
            deleted_at: deleted_at
                .map(|x| OffsetDateTime::parse(x, &Rfc3339))
                .transpose()?,
            version: SyncVersion::from(version),
        })
    }
}
