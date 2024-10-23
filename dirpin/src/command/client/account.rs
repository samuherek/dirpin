use clap::Parser;

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct RegisterCmd {
    u: String,
    e: String,
    p: String,
}

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub struct LoginCmd {
    u: String,
    p: String,
    k: String,
}

#[derive(Parser, Debug)]
#[clap(infer_subcommands = true)]
pub enum Cmd {
    Login(LoginCmd),
    Register(RegisterCmd),
    Logout,
    Delete,
}

impl Cmd {
    pub(crate) fn run(self) {
        println!("Account");
        match self {
            Self::Register(_) => todo!("Register"),
            Self::Login(_) => todo!("Login"),
            Self::Logout => todo!("Logout"),
            Self::Delete => todo!("Delete"),
        }
    }
}
