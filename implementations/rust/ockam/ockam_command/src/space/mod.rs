use clap::{Args, Subcommand};

pub use create::CreateCommand;
pub use delete::DeleteCommand;
pub use list::ListCommand;
use ockam::Context;
pub use show::ShowCommand;

use crate::{CommandGlobalOpts, HELP_TEMPLATE};

mod create;
mod delete;
mod list;
mod show;

#[derive(Clone, Debug, Args)]
pub struct SpaceCommand {
    #[clap(subcommand)]
    subcommand: SpaceSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpaceSubcommand {
    /// Create spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),

    /// Delete spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Delete(DeleteCommand),

    /// List spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    List(ListCommand),

    /// Show spaces
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Show(ShowCommand),
}

impl SpaceCommand {
    pub async fn run(ctx: &mut Context, opts: CommandGlobalOpts, cmd: SpaceCommand) -> anyhow::Result<()> {
        match cmd.subcommand {
            SpaceSubcommand::Create(scmd) => CreateCommand::run(opts, scmd),
            SpaceSubcommand::Delete(scmd) => DeleteCommand::run(opts, scmd),
            SpaceSubcommand::List(scmd) => ListCommand::run(opts, scmd),
            SpaceSubcommand::Show(scmd) => ShowCommand::run(ctx, opts, scmd).await?,
        }
        Ok(())
    }
}
