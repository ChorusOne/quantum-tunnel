use crate::config::CosmosConfig;
use crate::cosmos::types::TMHeader;
use crate::substrate::types::SignedBlockWithAuthoritySet;
use futures::executor::block_on;
use futures::{
    channel::mpsc::{UnboundedReceiver as Receiver, UnboundedSender as Sender},
    try_join,
};
use simple_error::SimpleError;
use std::error::Error;
use std::fmt;
use tendermint::net::Address;
use tendermint_rpc::{
    event_listener::{EventListener, EventSubscription, TMEventData::EventDataNewBlock},
    Client,
};
use url::Url;

pub struct CosmosHandler {
    socket: EventListener,
    client: Client,
    out_chan: Option<Sender<TMHeader>>,
    in_chan: Option<Receiver<SignedBlockWithAuthoritySet>>,
    cfg: CosmosConfig,
}

impl CosmosHandler {
    pub async fn new(cfg: CosmosConfig) -> Result<Self, Box<dyn Error>> {
        let rpc_url = Url::parse(&cfg.rpc_addr)?;

        let tm_addr = Address::Tcp {
            host: rpc_url.host_str().unwrap().to_string(),
            port: rpc_url.port().unwrap(),
            peer_id: None,
        };
        let client = Client::new(tm_addr.clone());
        println!("open websocket to to {:?}", tm_addr.clone());
        let socket = EventListener::connect(tm_addr.clone())
            .await
            .map_err(|e| SimpleError::new(e.to_string()))?;

        println!("connected websocket to {:?}", tm_addr.clone());
        Ok(CosmosHandler {
            socket: socket,
            client: client,
            out_chan: None,
            in_chan: None,
            cfg: cfg,
        })
    }

    /// Subscribes to new blocks from Websocket, and pushes TMHeader objects into the Channel.
    pub async fn recv_handler(&mut self, outchan: Sender<TMHeader>) {
        self.socket
            .subscribe(EventSubscription::BlockSubscription)
            .await;
        loop {
            let result = self.socket.get_event().await;
            match result {
                Err(e) => {
                    return;
                } // TODO: handle errors properly.
                Ok(res) => match res {
                    None => println!("No block"),
                    Some(block) => {
                        match block.data {
                            EventDataNewBlock(e) => {
                                match e.block {
                                    Some(block) => {
                                        // TODO: better naming of some of these return values!
                                        let commit_fut = self.client.commit(block.header.height);
                                        let vs_fut = self.client.validators(block.header.height);
                                        let r2 = try_join!(commit_fut, vs_fut);
                                        match r2 {
                                            Err(e) => {
                                                return;
                                            }
                                            Ok(r3) => {
                                                let h = TMHeader {
                                                    signed_header: r3.0.signed_header,
                                                    validator_set: r3.1.validators,
                                                };
                                                outchan.unbounded_send(h);
                                                println!(
                                                    "Processed incoming tendermint block for {:}",
                                                    block.header.height
                                                );
                                            }
                                        }
                                    }
                                    None => println!("No block (2)"), // TODO: handle errors properly.
                                }
                            }
                            _ => println!("Unexpected type"), // TODO: handle errors properly.
                        }
                    }
                },
            }
        }
    }

    pub fn send_handler(&self, cfg: CosmosConfig, inchan: Receiver<SignedBlockWithAuthoritySet>) {}

    pub fn create_client(&self, cfg: CosmosConfig, block: SignedBlockWithAuthoritySet) {}
}
