use clap::Args;

use ockam::TcpTransport;

use crate::node::show::print_query_status;
use crate::util::{node_rpc, RpcBuilder};
use crate::{help, node::HELP_DETAIL, util::startup::spawn_node, CommandGlobalOpts};

/// Start Nodes
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct StartCommand {
    /// Name of the node.
    #[arg(default_value = "default")]
    node_name: String,

    #[arg(long, default_value = "false")]
    aws_kms: bool,
}

impl StartCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, StartCommand),
) -> crate::Result<()> {
    let node_name = &cmd.node_name;
    let node_state = opts.state.nodes.get(node_name)?;
    node_state.kill_process(false)?;
    let node_setup = node_state.setup()?;

    // Restart node
    spawn_node(
        &opts,
        node_setup.verbose, // Previously user-chosen verbosity level
        true,               // skip-defaults because the node already exists
        node_name,          // The selected node name
        &node_setup.default_tcp_listener()?.addr.to_string(), // The selected node api address
        None,               // No project information available
        None,               // No invitation code available
        cmd.aws_kms,
    )?;

    // Print node status
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
    print_query_status(&mut rpc, node_name, true).await?;

    Ok(())
}
