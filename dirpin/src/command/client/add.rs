use clap::Parser;
use dirpin_client::database::{current_context, Database};
use dirpin_client::domain::Pin;
use dirpin_client::settings::Settings;
use dirpin_common::utils;
use eyre::{bail, Context, Result};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Cmd {
    value: Option<String>,
}

impl Cmd {
    pub async fn run(self, settings: &Settings, db: &Database) -> Result<()> {
        let context = current_context();
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
            return bail!("No input provided. Please run '--help' to see instructions.");
        };

        let pin = Pin::new(input, context.hostname, context.cwd, context.cgd);
        db.save(&pin).await?;

        println!("Pin added");
        Ok(())
    }
}
