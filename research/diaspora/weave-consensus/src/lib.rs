//! Checkpoint finality and proof-of-work fork choice for validated Weave frontiers.
//! The caller supplies ledger-validated frontier commitments and verified work.
#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub mod tapestry;

pub type Id = [u8; 32];

const NETWORK_MAGIC: &[u8] = b"DIASPORA:WEAVE:NETWORK:V1\0";
const NETWORK_FIXED_FIELDS: usize = 32 + 32 + 8 + 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub height: u64,
    pub id: Id,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frontier {
    pub id: Id,
    pub parent: Id,
    pub work: u128,
}

/// One canonical history of completed Weave frontiers, excluding genesis.
/// Each ID must commit to its complete frontier and preceding frontier.
/// The ledger validator must check PoW, unique proof accounting, transactions,
/// and commitment correctness before constructing this input.
#[derive(Debug)]
pub struct ValidatedHistory {
    frontiers: Vec<Frontier>,
    work: u128,
    genesis: Id,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    InvalidNetwork,
    InvalidHistory,
    InvalidCheckpoint,
    InvalidCertificate,
    MissingHistory,
    ConflictingFinality,
}

impl ValidatedHistory {
    pub fn from_validated_frontiers(genesis: Id, frontiers: Vec<Frontier>) -> Result<Self, Error> {
        let mut seen = BTreeSet::from([genesis]);
        let mut parent = genesis;
        let mut work = 0u128;
        for frontier in &frontiers {
            if frontier.parent != parent || frontier.work == 0 || !seen.insert(frontier.id) {
                return Err(Error::InvalidHistory);
            }
            work = work
                .checked_add(frontier.work)
                .ok_or(Error::InvalidHistory)?;
            parent = frontier.id;
        }
        Ok(Self {
            frontiers,
            work,
            genesis,
        })
    }

    fn id_at(&self, height: u64) -> Option<Id> {
        if height == 0 {
            return Some(self.genesis);
        }
        self.frontiers
            .get(usize::try_from(height - 1).ok()?)
            .map(|frontier| frontier.id)
    }

    pub fn tip(&self) -> Id {
        self.frontiers
            .last()
            .map_or(self.genesis, |frontier| frontier.id)
    }

    fn score(&self) -> (u128, std::cmp::Reverse<Id>) {
        let tip = self.tip();
        (self.work, std::cmp::Reverse(tip))
    }
}

pub struct Network {
    chain_id: Id,
    genesis: Checkpoint,
    interval: u64,
    validators: Vec<VerifyingKey>,
    quorum: usize,
}

impl Network {
    /// A fixed, equal-weight committee. Ledger rules ID identifies the agreed
    /// version of ledger validation independently of fork choice.
    pub fn new(
        ledger_rules_id: Id,
        genesis: Id,
        interval: u64,
        validators: Vec<VerifyingKey>,
    ) -> Result<Self, Error> {
        let distinct: BTreeSet<_> = validators
            .iter()
            .map(|key| key.to_montgomery().to_bytes())
            .collect();
        if interval == 0
            || validators.len() < 4
            || validators.len() > 256
            || distinct.len() != validators.len()
            || validators.iter().any(VerifyingKey::is_weak)
        {
            return Err(Error::InvalidNetwork);
        }
        let quorum = (2 * validators.len()) / 3 + 1;
        let mut manifest = Sha256::new();
        manifest.update(NETWORK_MAGIC);
        manifest.update(ledger_rules_id);
        manifest.update(genesis);
        manifest.update(interval.to_be_bytes());
        manifest.update((validators.len() as u16).to_be_bytes());
        for key in &validators {
            manifest.update(key.to_bytes());
        }
        let chain_id = manifest.finalize().into();
        Ok(Self {
            chain_id,
            genesis: Checkpoint {
                height: 0,
                id: genesis,
            },
            interval,
            validators,
            quorum,
        })
    }

    /// Parse the exact network file. No trailing data or alternate encodings.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let fields = bytes
            .strip_prefix(NETWORK_MAGIC)
            .ok_or(Error::InvalidNetwork)?;
        if fields.len() < NETWORK_FIXED_FIELDS {
            return Err(Error::InvalidNetwork);
        }
        let ledger_rules_id = fields[..32].try_into().map_err(|_| Error::InvalidNetwork)?;
        let genesis = fields[32..64]
            .try_into()
            .map_err(|_| Error::InvalidNetwork)?;
        let interval = u64::from_be_bytes(
            fields[64..72]
                .try_into()
                .map_err(|_| Error::InvalidNetwork)?,
        );
        let count = u16::from_be_bytes([fields[72], fields[73]]) as usize;
        let keys = &fields[NETWORK_FIXED_FIELDS..];
        if !(4..=256).contains(&count) || keys.len() != count * 32 {
            return Err(Error::InvalidNetwork);
        }
        let validators = keys
            .chunks_exact(32)
            .map(|key| {
                let key = key.try_into().map_err(|_| Error::InvalidNetwork)?;
                VerifyingKey::from_bytes(key).map_err(|_| Error::InvalidNetwork)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(ledger_rules_id, genesis, interval, validators)
    }

    pub fn chain_id(&self) -> Id {
        self.chain_id
    }

    /// Fixed-width, domain-separated signing preimage.
    pub fn vote_bytes(&self, parent: Checkpoint, target: Checkpoint) -> Vec<u8> {
        let mut bytes = b"DIASPORA:WEAVE:CHECKPOINT:V1\0".to_vec();
        for field in [
            &self.chain_id[..],
            &self.genesis.id,
            &parent.height.to_be_bytes(),
            &parent.id,
            &target.height.to_be_bytes(),
            &target.id,
        ] {
            bytes.extend_from_slice(field);
        }
        bytes
    }

    fn verify(&self, certificate: &Certificate) -> Result<(), Error> {
        if certificate.parent.height.checked_add(self.interval) != Some(certificate.target.height)
            || certificate.target.id == certificate.parent.id
        {
            return Err(Error::InvalidCheckpoint);
        }
        if !(self.quorum..=self.validators.len()).contains(&certificate.votes.len()) {
            return Err(Error::InvalidCertificate);
        }
        let bytes = self.vote_bytes(certificate.parent, certificate.target);
        let mut seen = BTreeSet::new();
        for vote in &certificate.votes {
            let validator = self
                .validators
                .get(vote.validator)
                .ok_or(Error::InvalidCertificate)?;
            if !seen.insert(vote.validator) {
                return Err(Error::InvalidCertificate);
            }
            validator
                .verify_strict(&bytes, &vote.signature)
                .map_err(|_| Error::InvalidCertificate)?;
        }
        Ok(())
    }
}

pub struct Vote {
    pub validator: usize,
    pub signature: Signature,
}

pub struct Certificate {
    pub parent: Checkpoint,
    pub target: Checkpoint,
    pub votes: Vec<Vote>,
}

/// Research state. A real node must durably persist and reload the checkpoint
/// history and halt flag, and process certificates serially.
pub struct ForkChoice {
    network: Network,
    checkpoints: BTreeMap<u64, Id>,
    finalized: Checkpoint,
    halted: bool,
}

impl ForkChoice {
    pub fn new(network: Network) -> Self {
        let finalized = network.genesis;
        Self {
            network,
            checkpoints: BTreeMap::from([(0, finalized.id)]),
            finalized,
            halted: false,
        }
    }

    pub fn finalized(&self) -> Checkpoint {
        self.finalized
    }

    pub fn apply(
        &mut self,
        certificate: &Certificate,
        history: Option<&ValidatedHistory>,
    ) -> Result<(), Error> {
        if self.halted {
            return Err(Error::ConflictingFinality);
        }
        self.network.verify(certificate)?;
        if self.checkpoints.get(&certificate.parent.height) != Some(&certificate.parent.id) {
            return Err(Error::InvalidCheckpoint);
        }
        if let Some(previous) = self.checkpoints.get(&certificate.target.height) {
            if *previous != certificate.target.id {
                self.halted = true;
                return Err(Error::ConflictingFinality);
            }
            return Ok(());
        }
        let history = history.ok_or(Error::MissingHistory)?;
        if history.genesis != self.network.genesis.id
            || history.id_at(self.finalized.height) != Some(self.finalized.id)
            || history.id_at(certificate.target.height) != Some(certificate.target.id)
        {
            return Err(Error::InvalidHistory);
        }
        self.finalized = certificate.target;
        self.checkpoints
            .insert(self.finalized.height, self.finalized.id);
        Ok(())
    }

    fn compatible(&self, history: &ValidatedHistory) -> bool {
        history.genesis == self.network.genesis.id
            && history.id_at(self.finalized.height) == Some(self.finalized.id)
    }

    pub fn select<'a>(
        &self,
        histories: &'a [ValidatedHistory],
    ) -> Result<Option<&'a ValidatedHistory>, Error> {
        self.select_iter(histories.iter())
    }

    fn select_iter<'a>(
        &self,
        histories: impl Iterator<Item = &'a ValidatedHistory>,
    ) -> Result<Option<&'a ValidatedHistory>, Error> {
        if self.halted {
            return Err(Error::ConflictingFinality);
        }
        Ok(histories
            .filter(|history| self.compatible(history))
            .max_by_key(|history| history.score()))
    }
}
