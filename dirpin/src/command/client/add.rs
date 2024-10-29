use clap::Parser;
use dirpin_client::settings::Settings;
use eyre::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Cmd {
    value: String,
}

impl Cmd {
    pub async fn run(self, settings: &Settings) -> Result<()> {
        let file_db_path = PathBuf::from(&settings.db_path)
            .parent()
            .expect("Failed to get the parent of the db")
            .join("temp_db");

        if let Some(parent) = file_db_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).wrap_err("Failed to create parent directories")?;
            }
        }

        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_db_path)
            .wrap_err("Failed to open file")?;
        file.write(format!("{}\n", self.value).as_bytes())
            .wrap_err("Failed to write to the file")?;

        println!("Pin added {}", self.value);
        Ok(())
    }
}
