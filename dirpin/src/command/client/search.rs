use clap::Parser;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {
    i: bool,
}

impl Cmd {
    pub(crate) fn run(self) {
        println!("Search");
    }
}
