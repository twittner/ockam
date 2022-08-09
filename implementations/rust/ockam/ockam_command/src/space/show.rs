use clap::Args;

use ockam::Context;
use ockam_api::cloud::space::Space;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, Rpc};
use crate::{CommandGlobalOpts, OutputFormat};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Id of the space.
    #[clap(display_order = 1001)]
    pub id: String,

    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub async fn run(ctx: &mut Context, opts: CommandGlobalOpts, cmd: ShowCommand) -> anyhow::Result<()> {
        let cfg = opts.config.get_node(&cmd.node_opts.api_node)?;
        let mut rpc = Rpc::new(ctx);
        rpc.request(&cfg, api::space::show(cmd)).await?;
        let space: Space = rpc.response()?;
        match opts.global_args.output_format {
            OutputFormat::Plain => println!("{space:#?}"),
            OutputFormat::Json => println!("{}", serde_json::to_string(&space)?)
        }
        Ok(())
    }
}
