use clap::Parser;
use eyre::Result;
use command::DirpinCmd;
mod command;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    author = "Sam Uherek",
    version = VERSION,
    )]
struct Dirpin {
    #[command(subcommand)]
    dirpin: DirpinCmd,
}

impl Dirpin {
    fn run(self) -> Result<()> {
        self.dirpin.run()
    }
}

fn main() -> Result<()> {
    Dirpin::parse().run()
}
