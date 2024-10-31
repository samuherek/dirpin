use clap::Parser;
use dirpin_client::database::Database;
use dirpin_client::settings::Settings;
use eyre::Result;

#[derive(Debug, Parser)]
pub struct Cmd {
    #[arg(short, long)]
    force: bool,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings, db: &Database) -> Result<()> {
        dirpin_client::sync::sync(settings, db, self.force).await?;
        Ok(())
    }
}
