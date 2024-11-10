use clap::Parser;
use dirpin_client::database::{current_context, Database, FilterMode};
use dirpin_client::settings::Settings;
use eyre::Result;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {
    #[arg(short, long)]
    cwd: bool,
}

impl Cmd {
    pub(crate) async fn run(self, _settings: &Settings, db: &Database) -> Result<()> {
        let context = current_context();
        let pins = db.list(&[FilterMode::Workspace], &context, "").await?;

        for el in pins {
            let (_, user) = el.hostname.split_once(":").unwrap();
            println!("{}:: {}", user, el.note);
        }

        Ok(())
    }
}
