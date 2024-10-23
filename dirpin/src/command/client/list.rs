use clap::Parser;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct Cmd {
    cwd: bool,
}

impl Cmd {
    pub(crate) fn run(self) {
        println!("List");
    }
}
