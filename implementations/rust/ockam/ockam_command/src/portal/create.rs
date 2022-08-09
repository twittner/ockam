use ockam_api::config::cli::NodeConfig;
use crate::node::NodeOpts;
use crate::util::{api, connect};
use crate::util::{ComposableSnippet, Operation, PortalMode, Protocol};
use crate::CommandGlobalOpts;
use clap::{Args, Subcommand};
use ockam::{Context, Address};
use ockam_api::error::ApiError;
use ockam_api::{
    nodes::models::portal::{InletStatus, OutletStatus},
    nodes::NODEMANAGER_ADDR,
    Status,
};
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    /// Select a creation variant
    #[clap(subcommand)]
    pub create_subcommand: CreateTypeCommand,

    /// Give this portal endpoint a name.  If none is provided a
    /// random one will be generated.
    pub alias: Option<String>,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        let bind = cc.create_subcommand.bind();
        let peer = cc.create_subcommand.peer();
        let mode = cc.create_subcommand.mode();

        Self {
            id: format!("_portal_{}_{}_{}_{}", mode, "tcp", bind, peer,),
            op: Operation::Portal {
                mode,
                protocol: Protocol::Tcp,
                bind,
                peer,
            },
            params: vec![],
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum CreateTypeCommand {
    /// Create a TCP portal inlet
    TcpInlet {
        /// Portal inlet bind address
        bind: String,
        /// Forwarding point for the portal (ockam routing address)
        outlet_addr: MultiAddr,
    },
    /// Create a TCP portal outlet
    TcpOutlet {
        /// Portal outlet connection address
        tcp_address: String,
        /// Portal outlet worker address
        worker_address: Address,
    },
}

impl CreateTypeCommand {
    fn mode(&self) -> PortalMode {
        match self {
            Self::TcpInlet { .. } => PortalMode::Inlet,
            Self::TcpOutlet { .. } => PortalMode::Outlet,
        }
    }

    fn bind(&self) -> String {
        match self {
            Self::TcpInlet { bind, .. } => bind.clone(),
            Self::TcpOutlet { worker_address, .. } => worker_address.to_string(),
        }
    }

    fn peer(&self) -> String {
        match self {
            Self::TcpInlet { outlet_addr, .. } => outlet_addr.to_string(),
            Self::TcpOutlet { tcp_address, .. } => tcp_address.clone(),
        }
    }
}

impl CreateCommand {
    pub async fn run(ctx: &mut Context, opts: CommandGlobalOpts, command: CreateCommand) -> anyhow::Result<()> {
        let nodecfg = opts.config.get_node(&command.node_opts.api_node)?;
        let composite = (&command).into();
        let node = command.node_opts.api_node.clone();

        match command.create_subcommand {
            CreateTypeCommand::TcpInlet { .. } => create_inlet(ctx, &nodecfg, command).await?,
            CreateTypeCommand::TcpOutlet { .. } => create_outlet(ctx, &nodecfg, command).await?,
        }

        // Update the startup config
        let startup_cfg = opts.config.get_launch_config(&node)?;
        startup_cfg.add_composite(composite);
        startup_cfg.atomic_update().run()?;
        Ok(())
    }
}

pub async fn create_inlet(ctx: &mut Context, cfg: &NodeConfig, cmd: CreateCommand) -> anyhow::Result<()> {
    let (bind, outlet_addr) = match cmd.create_subcommand {
        CreateTypeCommand::TcpInlet { bind, outlet_addr } => (bind, outlet_addr),
        CreateTypeCommand::TcpOutlet { .. } => {
            return Err(ApiError::generic("Internal logic error").into())
        }
    };

    let mut route = connect(ctx, cfg).await?;

    let resp: Vec<u8> = ctx
        .send_and_receive(
            route.modify().append(NODEMANAGER_ADDR),
            api::create_inlet(&bind, &outlet_addr, &cmd.alias)?,
        )
        .await?;

    let (
        response,
        InletStatus {
            bind_addr, alias, ..
        },
    ) = api::parse_inlet_status(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!(
                "Portal inlet '{}' created! You can send messages to it on this tcp address: \n{}`",
                alias, bind_addr
            )
        }

        _ => eprintln!("An unknown error occurred while creating an inlet..."),
    }

    Ok(())
}

pub async fn create_outlet(ctx: &mut Context, cfg: &NodeConfig, cmd: CreateCommand) -> anyhow::Result<()> {
    let (tcp_address, worker_address) = match cmd.create_subcommand {
        CreateTypeCommand::TcpInlet { .. } => {
            return Err(ApiError::generic("Internal logic error").into())
        }
        CreateTypeCommand::TcpOutlet {
            tcp_address,
            worker_address,
        } => (tcp_address, worker_address),
    };

    let mut route = connect(ctx, cfg).await?;

    let resp: Vec<u8> = ctx
        .send_and_receive(
            route.modify().append(NODEMANAGER_ADDR),
            api::create_outlet(&tcp_address, worker_address.to_string(), &cmd.alias)?,
        )
        .await?;

    let (
        response,
        OutletStatus {
            worker_addr, alias, ..
        },
    ) = api::parse_outlet_status(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!(
                "Portal outlet '{}' created! You can send messages through it via this address:\n{}",
                alias,
                worker_addr
            );
        }

        _ => eprintln!("An unknown error occurred while creating an outlet..."),
    }

    Ok(())
}
