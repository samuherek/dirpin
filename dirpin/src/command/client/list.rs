use clap::Parser;
use dirpin_client::settings::Settings;
use eyre::{Context, Result};
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {
    #[arg(short, long)]
    cwd: bool,
}

impl Cmd {
    pub(crate) fn run(self, settings: &Settings) -> Result<()> {
        let file_db_path = PathBuf::from(&settings.db_path)
            .parent()
            .expect("Failed to get the parent of the db")
            .join("temp_db");

        let data = read_to_string(file_db_path)
            .map(|x| x.lines().map(String::from).collect::<Vec<_>>())
            .wrap_err("Failed to read data")?;

        for item in data {
            println!("{item}");
        }

        Ok(())
    }
}
