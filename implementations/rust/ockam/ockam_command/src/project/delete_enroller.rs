use clap::Args;

use ockam::Context;

use crate::help;
use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// Remove an identity as authorized enroller from the project' authority
#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct DeleteEnrollerCommand {
    /// Id of the project.
    #[arg(display_order = 1001)]
    pub project_id: String,

    #[arg(display_order = 1002)]
    pub enroller_identity_id: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl DeleteEnrollerCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(|ctx| rpc(ctx, options, self));
    }
}

async fn rpc(
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: DeleteEnrollerCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    rpc.request(api::project::delete_enroller(&cmd)).await?;
    rpc.is_ok()?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
