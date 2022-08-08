use anyhow::Context;
use clap::Args;
use minicbor::Decoder;

use ockam_api::cloud::space::Space;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, node_api_request};
use crate::{CommandGlobalOpts, OutputFormat};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the space.
    #[clap(display_order = 1001)]
    pub name: String,

    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,

    /// Administrators for this space
    #[clap(display_order = 1100, last = true)]
    pub admins: Vec<String>,
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        let port = opts.config.get_node_port(&cmd.node_opts.api_node);
        node_api_request(port, opts, || async { api::space::create(cmd) }, create);
    }
}

fn create(dec: &mut Decoder<'_>, opts: CommandGlobalOpts) -> anyhow::Result<String> {
    let body = dec
        .decode::<Space>()
        .context("Failed to decode response body")?;
    let output = match opts.global_args.output_format {
        OutputFormat::Plain => body.id.to_string(),
        OutputFormat::Json => {
            serde_json::to_string(&body).context("Failed to serialize command output as json")?
        }
    };
    Ok(output)
}
