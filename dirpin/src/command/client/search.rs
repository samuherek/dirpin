use clap::Parser;
use eyre::Result;
use dirpin_client::interactive;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {}

impl Cmd {
    pub(crate) async fn run(self) -> Result<()> {
        interactive::run().await?;

        Ok(())
    }
}
