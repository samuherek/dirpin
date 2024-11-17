use clap::Parser;
use dirpin_client::database::Database;
use dirpin_client::domain::context::Context;
use dirpin_client::domain::entry::{Entry, EntryKind};
use dirpin_client::settings::Settings;
use dirpin_common::utils;
use eyre::{bail, Context as Ctx, Result};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct Cmd {
    value: Option<String>,

    #[arg(short, long)]
    global: bool,

    #[arg(long("type"), short('t'), name("type"))]
    kind: Option<String>,
}

impl Cmd {
    pub async fn run(self, settings: &Settings, db: &Database) -> Result<()> {
        let context = if self.global {
            Context::global(settings)
        } else {
            Context::cwd(settings)
        };
        let db_path = PathBuf::from(&settings.db_path);

        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                fs_err::create_dir_all(parent).wrap_err("Failed to create parent directories")?;
            }
        }

        let input = if let Some(value) = self.value {
            value
        } else if let Some(value) = utils::read_pipe_value()? {
            value
        } else {
            bail!("No input provided. Please run '--help' to see instructions.");
        };

        let mut entry = Entry::new(input, &context);

        if let Some(kind) = self.kind.map(|x| EntryKind::from_str(&x).unwrap()) {
            entry = entry.kind(kind);
        }

        db.save(&entry).await?;

        println!("Entry added");
        Ok(())
    }
}
