use clap::Parser;
use dirpin_client::database::Database;
use dirpin_client::domain::context::Context;
use dirpin_client::domain::entry::{Entry, EntryKind};
use dirpin_client::domain::workspace::Workspace;
use dirpin_client::settings::Settings;
use dirpin_common::utils;
use eyre::{bail, Context as Ctx, Result};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct Cmd {
    value: Option<String>,

    #[arg(short, long)]
    global: bool,

    #[arg(long("type"), short('t'), name("type"))]
    kind: Option<String>,
}

impl Cmd {
    pub async fn run(self, settings: &Settings, db: &Database) -> Result<()> {
        // TODO: move this into db init.
        let db_path = PathBuf::from(&settings.db_path);
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                fs_err::create_dir_all(parent).wrap_err("Failed to create parent directories")?;
            }
        }

        let input = if let Some(value) = self.value {
            value
        } else if let Some(value) = utils::read_pipe_value()? {
            value
        } else {
            bail!("No input provided. Please run '--help' to see instructions.");
        };

        let (context, workspace) = if self.global {
            let context = Context::global(settings);
            let global_name = Some("global".into());
            let workspace = match db.workspace(None, global_name, &context).await? {
                Some(ws) => ws,
                None => {
                    let ws = Workspace::new("global".into(), &context);
                    db.save_workspace(&ws).await?;
                    ws
                }
            };
            (context, Some(workspace))
        } else {
            let context = Context::cwd(settings);
            let mut workspace = db.workspace(None, None, &context).await?;
            if context.git.is_some() && workspace.is_none() {
                let ws = Workspace::new(context.workspace_name(), &context);
                db.save_workspace(&ws).await?;
                workspace = Some(ws);
            }
            (context, workspace)
        };

        let mut entry = Entry::new(
            input,
            context.path,
            workspace.map(|x| x.id),
            context.host_id,
        );

        if let Some(kind) = self.kind.map(|x| EntryKind::from_str(&x).unwrap()) {
            entry = entry.kind(kind);
        }

        db.save(&entry).await?;

        println!("Entry added");
        Ok(())
    }
}
