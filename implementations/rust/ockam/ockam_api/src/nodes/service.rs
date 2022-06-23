//! Node Manager (Node Man, the superhero that we deserve)

use super::{
    iolets::{IoletList, IoletStatus},
    types::{CreateTransport, DeleteTransport},
};
use crate::error::ApiError;
use crate::{nodes::types::*, Method, Request, Response, ResponseBuilder, Status};

use ockam::remote::RemoteForwarder;
use ockam::{Address, Context, Result, Route, Routed, TcpTransport, Worker};
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::String};
use ockam_core::errcode::{Kind, Origin};
use ockam_identity::authenticated_storage::mem::InMemoryStorage;
use ockam_identity::{Identity, TrustEveryonePolicy};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;

use core::convert::Infallible;
use minicbor::{encode::Write, Decoder};

type Alias = String;

/// Generate a new alias for some user created extension
#[inline]
fn random_alias() -> String {
    Address::random_local().without_type().to_owned()
}

// TODO: Move to multiaddr implementation
fn invalid_multiaddr_error() -> ockam_core::Error {
    ockam_core::Error::new(Origin::Core, Kind::Invalid, "Invalid multiaddr")
}

// TODO: Move to multiaddr implementation
fn map_multiaddr_err(_err: ockam_multiaddr::Error) -> ockam_core::Error {
    invalid_multiaddr_error()
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeMan {
    node_name: String,
    api_transport_id: Alias,
    transports: BTreeMap<Alias, (TransportType, TransportMode, String)>,
    tcp_transport: TcpTransport,
    // FIXME: wow this is a terrible way to store data
    iolets: BTreeMap<(Alias, IoletType), (String, Option<Route>)>,
}

impl NodeMan {
    /// Create a new NodeMan with the node name from the ockam CLI
    pub fn new(
        node_name: String,
        api_transport: (TransportType, TransportMode, String),
        tcp_transport: TcpTransport,
    ) -> Self {
        let api_transport_id = random_alias();
        let iolets = BTreeMap::new();
        let mut transports = BTreeMap::new();
        transports.insert(api_transport_id.clone(), api_transport);

        Self {
            node_name,
            api_transport_id,
            transports,
            tcp_transport,
            iolets,
        }
    }
}

impl NodeMan {
    //////// Transports API ////////

    // FIXME: return a ResponseBuilder here too!
    fn get_transports(&self) -> Vec<TransportStatus<'_>> {
        self.transports
            .iter()
            .map(|(tid, (tt, tm, addr))| TransportStatus::new(*tt, *tm, addr, tid))
            .collect()
    }

    async fn add_transport(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<TransportStatus<'_>>> {
        let CreateTransport { tt, tm, addr, .. } = dec.decode()?;

        use {TransportMode::*, TransportType::*};

        info!(
            "Handling request to create a new transport: {}, {}, {}",
            tt, tm, addr
        );
        let addr = addr.to_string();

        let res = match (tt, tm) {
            (Tcp, Listen) => self
                .tcp_transport
                .listen(&addr)
                .await
                .map(|socket| socket.to_string()),
            (Tcp, Connect) => self
                .tcp_transport
                .connect(&addr)
                .await
                .map(|ockam_addr| ockam_addr.to_string()),
            _ => unimplemented!(),
        };

        let response = match res {
            Ok(_) => {
                let tid = random_alias();
                self.transports.insert(tid.clone(), (tt, tm, addr.clone()));
                Response::ok(req.id()).body(TransportStatus::new(tt, tm, addr, tid))
            }
            Err(msg) => Response::bad_request(req.id()).body(TransportStatus::new(
                tt,
                tm,
                msg.to_string(),
                "<none>".to_string(),
            )),
        };

        Ok(response)
    }

    async fn delete_transport(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let body: DeleteTransport = dec.decode()?;
        info!("Handling request to delete transport: {}", body.tid);

        let tid: Alias = body.tid.into();

        if self.api_transport_id == tid && !body.force {
            warn!("User requested to delete the API transport without providing force OP flag...");
            return Ok(Response::bad_request(req.id()));
        }

        match self.transports.get(&tid) {
            Some(t) if t.1 == TransportMode::Listen => {
                warn!("It is not currently supported to destroy LISTEN transports");
                Ok(Response::bad_request(req.id()))
            }
            Some(t) => {
                self.tcp_transport.disconnect(&t.2).await?;
                self.transports.remove(&tid);
                Ok(Response::ok(req.id()))
            }
            None => Ok(Response::bad_request(req.id())),
        }
    }

    //////// Forwarder API ////////

    async fn create_forwarder<W>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = Infallible>,
    {
        let CreateForwarder { address, alias, .. } = dec.decode()?;
        let address = Address::from_string(address.to_string());
        debug!(%address, ?alias, "Handling CreateForwarder request");
        let forwarder = match alias {
            Some(alias) => RemoteForwarder::create_static(ctx, address, alias.to_string()).await,
            None => RemoteForwarder::create(ctx, address).await,
        };
        match forwarder {
            Ok(info) => {
                let b = ForwarderInfo::from(info);
                debug!(
                    forwarding_route = %b.forwarding_route(),
                    remote_address = %b.remote_address(),
                    "CreateForwarder request processed, sending back response"
                );
                Response::ok(req.id()).body(b).encode(enc)?;
            }
            Err(err) => {
                error!(?err, "Failed to create forwarder");
                Response::builder(req.id(), Status::InternalServerError)
                    .body(err.to_string())
                    .encode(enc)?;
            }
        };
        Ok(())
    }

    //////// Secure channel API ////////

    async fn create_secure_channel(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<CreateSecureChannelResponse<'_>>> {
        let CreateSecureChannelRequest { addr, .. } = dec.decode()?;

        info!("Handling request to create a new secure channel: {}", addr);

        // TODO: Improve error handling
        let addr = MultiAddr::try_from(addr.as_ref()).map_err(map_multiaddr_err)?;
        let route = crate::multiaddr_to_route(&addr)
            .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

        // TODO: Load Vault and Identity from the Storage. Possibly move this part from ockam_command
        let identity = Identity::create(ctx, &Vault::create()).await?;

        let channel = identity
            .create_secure_channel(route, TrustEveryonePolicy, &InMemoryStorage::new())
            .await?;

        // TODO: Create Secure Channels Registry

        let response =
            Response::ok(req.id()).body(CreateSecureChannelResponse::new(channel.to_string()));

        Ok(response)
    }

    async fn create_secure_channel_listener(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let CreateSecureChannelListenerRequest { addr, .. } = dec.decode()?;

        info!(
            "Handling request to create a new secure channel listener: {}",
            addr
        );

        // TODO: Should we check if Address is LOCAL?
        let addr = Address::from(addr.as_ref());

        // TODO: Load Vault and Identity from the Storage. Possibly move this part from ockam_command
        let identity = Identity::create(ctx, &Vault::create()).await?;

        identity
            .create_secure_channel_listener(addr, TrustEveryonePolicy, &InMemoryStorage::new())
            .await?;

        // TODO: Create Secure Channel Listeners Registry

        let response = Response::ok(req.id());

        Ok(response)
    }

    //////// Inlet and Outlet portal API ////////

    fn get_iolets(&self, req: &Request<'_>) -> ResponseBuilder<IoletList<'_>> {
        Response::ok(req.id()).body(IoletList::new(
            self.iolets
                .iter()
                .map(|((alias, tt), (addr, route))| {
                    IoletStatus::new(
                        *tt,
                        addr,
                        alias,
                        route.as_ref().map(|r| r.to_string().into()),
                    )
                })
                .collect(),
        ))
    }

    async fn create_iolet(
        &mut self,
        _ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<IoletStatus<'_>>> {
        let CreateIolet {
            addr,
            alias,
            fwd,
            tt,
            ..
        } = dec.decode()?;
        let addr = addr.to_string();
        let alias = alias.map(|a| a.into()).unwrap_or_else(random_alias);

        let res = match tt {
            IoletType::Inlet => {
                info!("Handling request to create inlet portal");
                let outlet_route = Route::parse(fwd.unwrap()).unwrap();
                self.tcp_transport
                    .create_inlet(addr.clone(), outlet_route)
                    .await
                    .map(|(addr, _)| addr)
            }
            IoletType::Outlet => {
                info!("Handling request to create outlet portal");
                let self_addr = Address::random_local();
                self.tcp_transport
                    .create_outlet(self_addr.clone(), addr.clone())
                    .await
                    .map(|_| self_addr)
            }
        };

        Ok(match res {
            Ok(addr) => {
                Response::ok(req.id()).body(IoletStatus::new(tt, addr.to_string(), alias, None))
            }
            Err(e) => Response::bad_request(req.id()).body(IoletStatus::new(
                tt,
                addr,
                alias,
                Some(e.to_string().into()),
            )),
        })
    }

    //////// Request matching and response handling ////////

    async fn handle_request<W>(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = Infallible>,
    {
        trace! {
            target: "ockam::nodeman::service",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        use Method::*;
        let path = req.path();
        let method = match req.method() {
            Some(m) => m,
            None => todo!(),
        };

        match (method, path) {
            // ==*== Basic node information ==*==

            // TODO: create, delete, destroy remote nodes
            (Get, "/node") => Response::ok(req.id())
                .body(NodeStatus::new(
                    self.node_name.as_str(),
                    "[✓]",
                    ctx.list_workers().await?.len() as u32,
                    std::process::id() as i32,
                    self.transports.len() as u32,
                ))
                .encode(enc)?,

            // ==*== Transports ==*==

            // TODO: Get all transports
            (Get, "/node/transport") => Response::ok(req.id())
                .body(TransportList::new(self.get_transports()))
                .encode(enc)?,
            (Post, "/node/transport") => self.add_transport(req, dec).await?.encode(enc)?,
            (Delete, "/node/transport") => self.delete_transport(req, dec).await?.encode(enc)?,

            // ==*== Secure channels
            (Post, "/node/secure_channel") => self
                .create_secure_channel(ctx, req, dec)
                .await?
                .encode(enc)?,
            (Post, "/node/secure_channel_listener") => self
                .create_secure_channel_listener(ctx, req, dec)
                .await?
                .encode(enc)?,

            // ==*== Forwarder commands
            (Post, "/node/forwarder") => self.create_forwarder(ctx, req, dec, enc).await?,

            // ==*== Inlets & Outlets ==*==
            (Get, "/node/iolets") => self.get_iolets(req).encode(enc)?,
            (Post, "/node/iolets") => self.create_iolet(ctx, req, dec).await?.encode(enc)?,
            (Delete, "/node/iolets") => todo!(),

            // ==*== Catch-all for Unimplemented APIs ==*==
            (method, path) => {
                warn!("Called invalid endpoint: {} {}", method, path);
                todo!()
            }
        }

        Ok(())
    }
}

#[ockam::worker]
impl Worker for NodeMan {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Vec<u8>>) -> Result<()> {
        let mut buf = vec![];
        let mut dec = Decoder::new(msg.as_body());
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        self.handle_request(ctx, &req, &mut dec, &mut buf).await?;
        ctx.send(msg.return_route(), buf).await
    }
}
