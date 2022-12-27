use clap::Args;
use ockam::Context;

use ockam_api::cloud::project::Project;

use crate::node::util::delete_embedded_node;
use crate::project::util::config;
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc, Rpc};
use crate::CommandGlobalOpts;

/// List projects
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
    rpc.request(api::project::list(&cmd.cloud_opts.route()))
        .await?;
    let projects = rpc.parse_and_print_response::<Vec<Project>>()?;
    config::set_projects(&opts.config, &projects).await?;
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
