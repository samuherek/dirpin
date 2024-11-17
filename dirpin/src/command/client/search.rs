use clap::Parser;
use dirpin_client::database::Database;
use dirpin_client::domain::context::Context;
use dirpin_client::settings::Settings;
use eyre::Result;

mod interactive;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {}

impl Cmd {
    pub(crate) async fn run(self, settings: &Settings, database: &Database) -> Result<()> {
        let context = Context::cwd(settings);
        interactive::run(settings, database, &context).await?;

        Ok(())
    }
}
