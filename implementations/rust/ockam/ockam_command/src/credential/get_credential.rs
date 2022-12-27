use clap::Args;

use ockam::Context;

use crate::node::NodeOpts;
use crate::util::{api, node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct GetCredentialCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    #[arg(long)]
    pub overwrite: bool,
}

impl GetCredentialCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(|ctx| rpc(ctx, options, self));
    }
}

async fn rpc(
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: GetCredentialCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::background(&ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::credentials::get_credential(cmd.overwrite))
        .await?;
    Ok(())
}
