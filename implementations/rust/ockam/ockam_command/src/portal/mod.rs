mod create;
pub(crate) use create::CreateCommand;
use ockam::Context;

// TODO: add delete, list, show subcommands

use crate::{CommandGlobalOpts, HELP_TEMPLATE};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct PortalCommand {
    #[clap(subcommand)]
    subcommand: PortalSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PortalSubCommand {
    /// Create portals on the selected node
    #[clap(display_order = 900, help_template = HELP_TEMPLATE)]
    Create(CreateCommand),
}

impl PortalCommand {
    pub async fn run(ctx: &mut Context, opts: CommandGlobalOpts, cmd: PortalCommand) -> anyhow::Result<()> {
        match cmd.subcommand {
            PortalSubCommand::Create(cmd) => CreateCommand::run(ctx, opts, cmd).await,
        }
    }
}
