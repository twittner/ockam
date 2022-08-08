use clap::Args;
use minicbor::Decoder;

use ockam_api::cloud::space::Space;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, node_api_request};
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
    pub fn run(opts: CommandGlobalOpts, cmd: ShowCommand) {
        let port = opts.config.get_node_port(&cmd.node_opts.api_node);
        node_api_request(port, opts, || async { api::space::show(cmd) }, show);
    }
}

fn show(dec: &mut Decoder<'_>, opts: CommandGlobalOpts) -> anyhow::Result<String> {
    let body = dec.decode::<Space>()?;
    let output = match opts.global_args.output_format {
        OutputFormat::Plain => format!("{body:#?}"),
        OutputFormat::Json => serde_json::to_string(&body)?,
    };
    Ok(output)
}
