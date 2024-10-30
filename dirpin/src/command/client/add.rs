use clap::Parser;
use dirpin_client::database::{Database, current_context};
use dirpin_client::domain::Pin;
use dirpin_client::settings::Settings;
use eyre::{Context, Result};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Cmd {
    value: String,
}

impl Cmd {
    pub async fn run(self, settings: &Settings, db: &Database) -> Result<()> {
        let context = current_context();
        let file_db_path = PathBuf::from(&settings.db_path)
            .parent()
            .expect("Failed to get the parent of the db")
            .join("temp_db");

        if let Some(parent) = file_db_path.parent() {
            if !parent.exists() {
                fs_err::create_dir_all(parent).wrap_err("Failed to create parent directories")?;
            }
        }

        let pin = Pin::new(self.value, context.hostname, context.cwd, context.cgd);
        db.save(&pin).await?;

        println!("Pin added");
        Ok(())
    }
}
