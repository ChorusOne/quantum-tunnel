use crate::config::CeloConfig;
use crate::cosmos::types::TMHeader;
use crate::utils::prost_serialize;
use ethabi::Token;
use prost_types::Any;
use std::error::Error;
use web3::types::{H160, U256};

use crate::cosmos::proto::tendermint::light::{
    BlockIdFlag, CanonicalBlockId, CanonicalPartSetHeader, ClientState, Commit, CommitSig,
    Consensus, ConsensusState, Duration, Fraction, LightHeader, SignedHeader, Timestamp, TmHeader,
    Validator, ValidatorSet,
};

impl TMHeader {
    pub fn to_sol_create_msg(&self, cfg: &CeloConfig) -> Result<ethabi::Token, Box<dyn Error>> {
        let header = self.signed_header.header.to_owned();
        let height: u64 = header.height.into();

        let client_state = ClientState {
            chain_id: header.chain_id.to_string(),
            trust_level: Some(Fraction {
                numerator: 1,
                denominator: 3,
            }),
            trusting_period: Some(to_duration(parse_duration::parse(cfg.trusting_period.as_str())?.as_secs() as i64, 0)),
            unbonding_period: Some(to_duration(parse_duration::parse(cfg.unbonding_period.as_str())?.as_secs() as i64, 0)),
            max_clock_drift: Some(to_duration(parse_duration::parse(cfg.max_clock_drift.as_str())?.as_secs() as i64, 0)),
            frozen_height: 0,
            latest_height: header.height.into(),
            allow_update_after_expiry: cfg.allow_update_after_expiry,
            allow_update_after_misbehaviour: cfg.allow_update_after_misbehavior,
        };

        let consensus_state = ConsensusState {
            merkle_root_hash: header.app_hash.value(),
            timestamp: Some(to_timestamp(&header.time)),
            next_validators_hash: header.next_validators_hash.as_bytes().to_vec(),
        };

        let consensus_state_bytes = prost_serialize(&Any {
            type_url: "/tendermint.types.ConsensusState".to_string(),
            value: prost_serialize(&consensus_state)?,
        })?;

        let client_state_bytes = prost_serialize(&Any {
            type_url: "/tendermint.types.ClientState".to_string(),
            value: prost_serialize(&client_state)?,
        })?;

        // MsgCreateClient
        Ok(ethabi::Token::Tuple(vec![
            Token::String("07-tendermint".to_string()),
            Token::Uint(U256::from(height)),
            Token::Bytes(client_state_bytes),
            Token::Bytes(consensus_state_bytes),
        ]))
    }

    pub fn to_sol_update_msg(
        &self,
        cfg: &CeloConfig,
        validators: Vec<tendermint::validator::Info>,
        client_id: String,
    ) -> Result<ethabi::Token, Box<dyn Error>> {
        let signed_header = to_signed_header(&self.signed_header);
        let trusted_height: i64 = signed_header.header.as_ref().unwrap().height.into();

        let tm_header = TmHeader {
            signed_header: Some(signed_header.to_owned()),
            validator_set: Some(to_validator_set(&self.validator_set)),
            trusted_height: trusted_height - 1,
            trusted_validators: Some(to_validator_set(&validators)),
        };

        let serialized_header = prost_serialize(&Any {
            type_url: "/tendermint.types.TmHeader".to_string(),
            value: prost_serialize(&tm_header)?,
        })?;

        // MsgUpdateClient
        Ok(ethabi::Token::Tuple(vec![
            Token::String(client_id.clone()),
            Token::Bytes(serialized_header),
        ]))
    }
}

pub fn to_part_set_header(
    part_set_header: &tendermint::block::parts::Header,
) -> CanonicalPartSetHeader {
    CanonicalPartSetHeader {
        total: part_set_header.total,
        hash: part_set_header.hash.as_bytes().to_vec(),
    }
}

pub fn to_block_id(last_block_id: &tendermint::block::Id) -> CanonicalBlockId {
    CanonicalBlockId {
        hash: last_block_id.hash.as_bytes().to_vec(),
        part_set_header: Some(to_part_set_header(&last_block_id.part_set_header)),
    }
}

pub fn to_timestamp(timestamp: &tendermint::time::Time) -> Timestamp {
    let t: tendermint_proto::google::protobuf::Timestamp = timestamp.to_owned().into();

    Timestamp {
        seconds: t.seconds,
        nanos: t.nanos,
    }
}

pub fn to_version(version: &tendermint::block::header::Version) -> Consensus {
    Consensus {
        block: version.block,
        app: version.app,
    }
}

pub fn to_sig(sig: &tendermint::block::commit_sig::CommitSig) -> CommitSig {
    match sig {
        tendermint::block::commit_sig::CommitSig::BlockIDFlagAbsent => CommitSig {
            block_id_flag: BlockIdFlag::Absent.into(),
            validator_address: Vec::new(),
            timestamp: None,
            signature: Vec::new(),
        },
        tendermint::block::commit_sig::CommitSig::BlockIDFlagNil {
            validator_address,
            timestamp,
            signature,
        } => CommitSig {
            block_id_flag: BlockIdFlag::Nil.into(),
            validator_address: validator_address.to_owned().into(),
            timestamp: Some(to_timestamp(&timestamp)),
            signature: signature.to_owned().into(),
        },
        tendermint::block::commit_sig::CommitSig::BlockIDFlagCommit {
            validator_address,
            timestamp,
            signature,
        } => CommitSig {
            block_id_flag: BlockIdFlag::Commit.into(),
            validator_address: validator_address.to_owned().into(),
            timestamp: Some(to_timestamp(&timestamp)),
            signature: signature.to_owned().into(),
        },
    }
}

pub fn to_signed_header(
    signed_header: &tendermint::block::signed_header::SignedHeader,
) -> SignedHeader {
    let header = &signed_header.header;
    let commit = &signed_header.commit;

    SignedHeader {
        header: Some(LightHeader {
            chain_id: header.chain_id.to_string(),
            time: Some(to_timestamp(&header.time)),
            height: header.height.into(),
            next_validators_hash: header.next_validators_hash.into(),
            validators_hash: header.validators_hash.into(),
            app_hash: header.app_hash.to_owned().into(),
            consensus_hash: header.consensus_hash.into(),
            data_hash: header.data_hash.unwrap().into(),
            evidence_hash: header.evidence_hash.unwrap().into(),
            last_block_id: Some(to_block_id(&header.last_block_id.unwrap())),
            last_commit_hash: header.last_commit_hash.unwrap().into(),
            last_results_hash: header.last_results_hash.unwrap().into(),
            proposer_address: header.proposer_address.into(),
            version: Some(to_version(&header.version)),
        }),
        commit: Some(Commit {
            height: commit.height.into(),
            round: commit.round.into(),
            block_id: Some(to_block_id(&commit.block_id)),
            signatures: commit.signatures.iter().map(|sig| to_sig(sig)).collect(),
        }),
    }
}

pub fn to_validator_set(validators: &[tendermint::validator::Info]) -> ValidatorSet {
    ValidatorSet {
        validators: validators
            .iter()
            .map(|validator| Validator {
                pub_key: validator.pub_key.as_bytes().to_vec(),
                voting_power: validator.power() as i64,
            })
            .collect(),
        total_voting_power: 0,
    }
}

pub fn to_light_block(signed_header: &SignedHeader, validator_set: &ValidatorSet) -> TmHeader {
    TmHeader {
        trusted_validators: None,
        trusted_height: 0,
        signed_header: Some(signed_header.to_owned()),
        validator_set: Some(validator_set.to_owned()),
    }
}

pub fn to_duration(seconds: i64, nanos: i32) -> Duration {
    Duration { seconds, nanos }
}

pub fn to_addr(address: String) -> H160 {
    let stripped: Vec<u8> = hex::decode(&address[2..address.len()]).unwrap();
    let mut addr: [u8; 20] = Default::default();
    addr.copy_from_slice(&stripped[0..20]);

    H160::from(&addr)
}
