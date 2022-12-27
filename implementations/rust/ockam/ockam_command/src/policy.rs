use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{help, CommandGlobalOpts, Result};
use clap::{Args, Subcommand};
use ockam::Context;
use ockam_abac::{Action, Policy, PolicyList, Resource};
use ockam_core::api::Request;

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide(), after_long_help = help::template(HELP_DETAIL))]
pub struct PolicyCommand {
    #[command(subcommand)]
    subcommand: PolicySubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum PolicySubcommand {
    Set {
        /// Node on which to start the tcp inlet.
        #[arg(long, display_order = 900, id = "NODE")]
        at: String,

        #[arg(short, long)]
        resource: Resource,

        #[arg(short, long, default_value = "handle_message")]
        action: Action,

        #[arg(short, long)]
        policy: Policy,
    },
    Get {
        /// Node on which to start the tcp inlet.
        #[arg(long, display_order = 900, id = "NODE")]
        at: String,

        #[arg(short, long)]
        resource: Resource,

        #[arg(short, long)]
        action: Action,
    },
    Delete {
        /// Node on which to start the tcp inlet.
        #[arg(long, display_order = 900, id = "NODE")]
        at: String,

        #[arg(short, long)]
        resource: Resource,

        #[arg(short, long)]
        action: Action,
    },
    List {
        /// Node on which to start the tcp inlet.
        #[arg(long, display_order = 900, id = "NODE")]
        at: String,

        #[arg(short, long)]
        resource: Resource,
    },
}

impl PolicyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(|ctx| rpc(ctx, opts, self))
    }
}

#[rustfmt::skip]
async fn rpc(ctx: Context, opts: CommandGlobalOpts,cmd: PolicyCommand) -> Result<()> {
    match cmd.subcommand {
        PolicySubcommand::Set { at, resource, action, policy } => {
            let node = extract_address_value(&at)?;
            let bdy = policy;
            let req = Request::post(policy_path(&resource, &action)).body(bdy);
            let mut rpc = Rpc::background(&ctx, &opts, &node)?;
            rpc.request(req).await?;
            rpc.is_ok()?
        }
        PolicySubcommand::Get { at, resource, action } => {
            let node = extract_address_value(&at)?;
            let req = Request::get(policy_path(&resource, &action));
            let mut rpc = Rpc::background(&ctx, &opts, &node)?;
            rpc.request(req).await?;
            let pol: Policy = rpc.parse_response()?;
            println!("{}", pol.expression())
        }
        PolicySubcommand::Delete { at, resource, action } => {
            let node = extract_address_value(&at)?;
            let req = Request::delete(policy_path(&resource, &action));
            let mut rpc = Rpc::background(&ctx, &opts, &node)?;
            rpc.request(req).await?;
            rpc.is_ok()?
        }
        PolicySubcommand::List { at, resource } => {
            let node = extract_address_value(&at)?;
            let req = Request::get(format!("/policy/{resource}"));
            let mut rpc = Rpc::background(&ctx, &opts, &node)?;
            rpc.request(req).await?;
            let pol: PolicyList = rpc.parse_response()?;
            for (a, p) in pol.policies() {
                println!("{resource}/{a}: {p}")
            }
        }
    }
    Ok(())
}

fn policy_path(r: &Resource, a: &Action) -> String {
    format!("/policy/{r}/{a}")
}
