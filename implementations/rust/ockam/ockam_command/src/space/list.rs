use clap::Args;

use ockam::Context;
use ockam_api::cloud::space::Space;

use crate::node::util::delete_embedded_node;
use crate::space::util::config;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(|ctx| rpc(ctx, options, self));
    }
}

async fn rpc(
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    rpc.request(api::space::list(&cmd.cloud_opts.route()))
        .await?;
    let spaces = rpc.parse_and_print_response::<Vec<Space>>()?;
    config::set_spaces(&opts.config, &spaces)?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
