use clap::Args;

use ockam::Context;

use crate::node::NodeOpts;
use crate::secure_channel::HELP_DETAIL;
use crate::util::api;
use crate::util::{node_rpc, Rpc};
use crate::{help, CommandGlobalOpts};

/// List Secure Channel Listeners
#[derive(Args, Clone, Debug)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct ListCommand {
    /// Node of which secure listeners shall be listed
    #[command(flatten)]
    node_opts: NodeOpts,
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
    let mut rpc = Rpc::background(&ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::list_secure_channel_listener()).await?;
    let res = rpc.parse_response::<Vec<String>>()?;

    println!(
        "Secure channel listeners for node `{}`:",
        &cmd.node_opts.api_node
    );
    for addr in res {
        println!("  {}", addr);
    }

    Ok(())
}
