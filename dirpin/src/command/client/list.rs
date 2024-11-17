use clap::Parser;
use dirpin_client::database::{Database, FilterMode};
use dirpin_client::domain::context::Context;
use dirpin_client::settings::Settings;
use eyre::Result;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {
    #[arg(short, long)]
    cwd: bool,
}

impl Cmd {
    pub(crate) async fn run(self, settings: &Settings, db: &Database) -> Result<()> {
        let context = Context::cwd(settings);
        let workspace = db.workspace(None, None, &context).await?;
        let entries = db
            .list(FilterMode::Workspace, &context, workspace.as_ref(), "")
            .await?;

        for el in entries {
            println!("{}", el.value);
        }

        Ok(())
    }
}
