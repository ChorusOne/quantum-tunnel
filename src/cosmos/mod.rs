pub mod crypto;
pub mod handler;
pub mod types;

pub type Handler = handler::CosmosHandler;

pub mod proto {
    tonic::include_proto!("gogoproto");
    tonic::include_proto!("google.protobuf");

    pub mod ics23 {
        tonic::include_proto!("ics23");
    }

    pub mod tendermint {
        pub mod version {
            tonic::include_proto!("tendermint.version");
        }

        pub mod crypto {
            tonic::include_proto!("tendermint.crypto");
        }

        pub mod abci {
            tonic::include_proto!("tendermint.abci");
        }

        pub mod types {
            tonic::include_proto!("tendermint.types");
        }
    }

    pub mod cosmos {
        pub mod crypto {
            pub mod multisig {
                pub mod v1beta1 {
                    tonic::include_proto!("cosmos.crypto.multisig.v1beta1");
                }
            }
        }

        pub mod base {
            pub mod query {
                pub mod v1beta1 {
                    tonic::include_proto!("cosmos.base.query.v1beta1");
                }
            }

            pub mod abci {
                pub mod v1beta1 {
                    tonic::include_proto!("cosmos.base.abci.v1beta1");
                }
            }

            pub mod v1beta1 {
                tonic::include_proto!("cosmos.base.v1beta1");
            }
        }
        pub mod tx {
            pub mod signing {
                pub mod v1beta1 {
                    tonic::include_proto!("cosmos.tx.signing.v1beta1");
                }
            }

            pub mod v1beta1 {
                tonic::include_proto!("cosmos.tx.v1beta1");
            }
        }

        pub mod auth {
            pub mod v1beta1 {
                tonic::include_proto!("cosmos.auth.v1beta1");
            }
        }
    }

    pub mod ibc {
        pub mod core {
            pub mod commitment {
                pub mod v1 {
                    tonic::include_proto!("ibc.core.commitment.v1");
                }
            }

            pub mod client {
                pub mod v1 {
                    tonic::include_proto!("ibc.core.client.v1");
                }
            }
        }

        pub mod lightclients {
            pub mod wasm {
                pub mod v1 {
                    tonic::include_proto!("ibc.lightclients.wasm.v1");
                }
            }
        }
    }
}
