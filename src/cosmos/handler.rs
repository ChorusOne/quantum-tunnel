use futures::{try_join, channel::mpsc::{UnboundedSender as Sender, UnboundedReceiver as Receiver}};
use crate::cosmos::types::TMHeader;
use crate::substrate::types::SignedBlockWithAuthoritySet;
use crate::config::CosmosConfig;
use tendermint_rpc::{Client, event_listener::{EventListener, EventSubscription, TMEventData::EventDataNewBlock}};
use tendermint::net::Address;
use url::Url;
use std::error::Error;
use std::fmt;
use simple_error::SimpleError;
use futures::executor::block_on;

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

        let tm_addr = Address::Tcp{host: rpc_url.host_str().unwrap().to_string(), port: rpc_url.port().unwrap(), peer_id: None};
        let client = Client::new(tm_addr.clone());
        println!("open websocket to to {:?}", tm_addr.clone());
        let socket = EventListener::connect(tm_addr.clone()).await.map_err(|e| SimpleError::new(e.to_string()))?;

        println!("connected websocket to {:?}", tm_addr.clone());
        Ok(CosmosHandler {
            socket: socket,
            client: client,
            out_chan: None,
            in_chan: None,
            cfg: cfg,
        })
    }

    /// Listens to
    pub async fn recv_handler(&mut self, outchan: Sender<TMHeader>) {
        self.socket.subscribe(EventSubscription::BlockSubscription).await;
        loop {
            let result = self.socket.get_event().await;
            match result {
                Err(e) => { return; }
                Ok(res) => match res {
                    None => println!("No block"),
                    Some(block) => {
                        match block.data {
                            EventDataNewBlock(e) => {
                                match e.block {
                                    Some(block) => {

                                        let commit_fut = self.client.commit(block.header.height);
                                        let vs_fut = self.client.validators(block.header.height);
                                        let r2 = try_join!(commit_fut, vs_fut);
                                        match r2 {
                                            Err(e) => { return; }
                                            Ok(r3) => {
                                                let h = TMHeader{
                                                    signed_header: r3.0.signed_header,
                                                    validator_set: r3.1.validators,
                                                };
                                                outchan.unbounded_send(h);
                                                println!("Processing incoming tendermint block for {:}", block.header.height);
                                            }
                                        }

                                    },
                                    None => println!("No block (2)")
                                }
                                //self.client.commit()
                            },
                            _ => println!("Unexpected type")
                        }
                    }
                }
            }
        }
    }

    pub fn send_handler(&self, cfg: CosmosConfig, inchan: Receiver<SignedBlockWithAuthoritySet>) {

    }

    pub fn create_client(&self, cfg: CosmosConfig, block: SignedBlockWithAuthoritySet) {

    }
}
