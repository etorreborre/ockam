use clap::Args;

use ockam::Context;
use ockam_multiaddr::MultiAddr;

use crate::node::NodeOpts;
use crate::util::api::{self};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct PresentCredentialCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    #[arg(long, display_order = 900, id = "ROUTE")]
    pub to: MultiAddr,

    #[arg(short, long)]
    pub oneway: bool,
}

impl PresentCredentialCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(|ctx| rpc(ctx, options, self));
    }
}

async fn rpc(
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: PresentCredentialCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::background(&ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::credentials::present_credential(&cmd.to, cmd.oneway))
        .await?;
    Ok(())
}
