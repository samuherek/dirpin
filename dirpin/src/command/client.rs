use clap::Parser;
use eyre::Result;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    Start,
}

impl Cmd {
    #[tokio::main]
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Start => dirpin_client::start_client()
                .await
                .expect("Failed to run client"),
        }
        Ok(())
    }
}
