use clap::Parser;
use dirpin::command::DirpinCmd;
use dirpin::VERSION;
use eyre::Result;

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
