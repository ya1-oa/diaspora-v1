use diaspora_weave_consensus::{
    Certificate, Checkpoint, Error, ForkChoice, Frontier, Network, ValidatedHistory, Vote,
};
use ed25519_dalek::{Signer, SigningKey};

const GENESIS: [u8; 32] = [0; 32];
const ROOT: Checkpoint = Checkpoint {
    height: 0,
    id: GENESIS,
};

fn point(height: u64, value: u8) -> Checkpoint {
    Checkpoint {
        height,
        id: [value; 32],
    }
}

fn keys() -> Vec<SigningKey> {
    (1..=4)
        .map(|value| SigningKey::from_bytes(&[value; 32]))
        .collect()
}

fn config(keys: &[SigningKey], rules: u8) -> Network {
    Network::new(
        [rules; 32],
        GENESIS,
        2,
        keys.iter().map(SigningKey::verifying_key).collect(),
    )
    .unwrap()
}

fn history(entries: &[(u8, u128)]) -> ValidatedHistory {
    let mut parent = GENESIS;
    let frontiers = entries
        .iter()
        .map(|&(value, work)| {
            let id = [value; 32];
            let frontier = Frontier { id, parent, work };
            parent = id;
            frontier
        })
        .collect();
    ValidatedHistory::from_validated_frontiers(GENESIS, frontiers).unwrap()
}

fn cert(
    network: &Network,
    keys: &[SigningKey],
    parent: Checkpoint,
    target: Checkpoint,
    voters: &[usize],
) -> Certificate {
    let bytes = network.vote_bytes(parent, target);
    let votes = voters
        .iter()
        .map(|&validator| Vote {
            validator,
            signature: keys[validator].sign(&bytes),
        })
        .collect();
    Certificate {
        parent,
        target,
        votes,
    }
}

#[test]
fn only_unfinalized_work_can_be_reorganized() {
    let keys = keys();
    let network = config(&keys, 1);
    let checkpoint = cert(&network, &keys, ROOT, point(2, 2), &[0, 1, 2]);
    let mut node = ForkChoice::new(network);
    let pre_checkpoint = [history(&[(1, 10), (2, 10)]), history(&[(3, 50), (4, 50)])];
    assert_eq!(
        node.select(&pre_checkpoint).unwrap().unwrap().tip(),
        [4; 32]
    );
    node.apply(&checkpoint, Some(&pre_checkpoint[0])).unwrap();
    for work in [100, 1_000, 10_000, 1_000_000] {
        let forks = [
            history(&[(1, 10), (2, 10), (5, 1)]),
            history(&[(3, work), (4, work)]),
            history(&[(1, 10), (2, 10), (6, 2)]),
        ];
        assert_eq!(node.select(&forks).unwrap().unwrap().tip(), [6; 32]);
        assert!(node.select(&forks[1..2]).unwrap().is_none());
    }
}

#[test]
fn ties_are_arrival_order_independent() {
    let node = ForkChoice::new(config(&keys(), 1));
    for order in [[2, 3], [3, 2]] {
        let histories = order.map(|id| history(&[(id, 10)]));
        assert_eq!(node.select(&histories).unwrap().unwrap().tip(), [2; 32]);
    }
}

#[test]
fn partition_below_quorum_stalls_finalization() {
    let keys = keys();
    let network = config(&keys, 1);
    let left = cert(&network, &keys, ROOT, point(2, 2), &[0, 1]);
    let right = cert(&network, &keys, ROOT, point(2, 4), &[2, 3]);
    let recovered = cert(&network, &keys, ROOT, point(2, 2), &[0, 1, 2]);
    let mut node = ForkChoice::new(network);
    for candidate in [&left, &right] {
        assert_eq!(node.apply(candidate, None), Err(Error::InvalidCertificate));
        assert_eq!(node.finalized(), ROOT);
    }
    assert_eq!(node.apply(&recovered, None), Err(Error::MissingHistory));
    assert_eq!(
        node.apply(&recovered, Some(&history(&[(3, 1), (4, 1)]))),
        Err(Error::InvalidHistory)
    );
    node.apply(&recovered, Some(&history(&[(1, 1), (2, 1)])))
        .unwrap();
    assert_eq!(node.finalized(), point(2, 2));
}

#[test]
fn conflicting_certificate_halts_even_after_a_later_checkpoint() {
    let keys = keys();
    let network = config(&keys, 1);
    let first = cert(&network, &keys, ROOT, point(2, 2), &[0, 1, 2]);
    let next = cert(&network, &keys, point(2, 2), point(4, 4), &[0, 1, 3]);
    let conflict = cert(&network, &keys, ROOT, point(2, 6), &[1, 2, 3]);
    let path = history(&[(1, 1), (2, 1), (3, 1), (4, 1)]);
    let mut node = ForkChoice::new(network);
    assert_eq!(
        node.apply(&next, Some(&path)),
        Err(Error::InvalidCheckpoint)
    );
    node.apply(&first, Some(&path)).unwrap();
    node.apply(&next, Some(&path)).unwrap();
    node.apply(&first, None).unwrap();
    assert_eq!(node.apply(&conflict, None), Err(Error::ConflictingFinality));
    assert_eq!(node.select(&[]).unwrap_err(), Error::ConflictingFinality);
    assert_eq!(node.finalized(), point(4, 4));
}

#[test]
fn rejects_duplicate_forged_and_other_network_votes() {
    let keys = keys();
    let network = config(&keys, 1);
    let duplicate = cert(&network, &keys, ROOT, point(2, 2), &[0, 0, 1]);
    let foreign = cert(&config(&keys, 2), &keys, ROOT, point(2, 2), &[0, 1, 2]);
    let mut forged = cert(&network, &keys, ROOT, point(2, 2), &[0, 1, 2]);
    forged.target.id = [5; 32];
    let skipped = cert(&network, &keys, ROOT, point(4, 4), &[0, 1, 2]);
    let mut node = ForkChoice::new(network);
    for invalid in [&duplicate, &foreign, &forged] {
        assert_eq!(node.apply(invalid, None), Err(Error::InvalidCertificate));
    }
    assert_eq!(node.apply(&skipped, None), Err(Error::InvalidCheckpoint));
}

#[test]
fn catches_invalid_history_structure_and_overflow() {
    let first = Frontier {
        id: [1; 32],
        parent: GENESIS,
        work: u128::MAX,
    };
    let second = Frontier {
        id: [2; 32],
        parent: first.id,
        work: 1,
    };
    let disconnected = Frontier {
        parent: GENESIS,
        ..second
    };
    let repeated = Frontier {
        id: first.id,
        ..second
    };
    let zero_work = Frontier { work: 0, ..second };
    for entry in [second, disconnected, repeated, zero_work] {
        assert_eq!(
            ValidatedHistory::from_validated_frontiers(GENESIS, vec![first, entry]).unwrap_err(),
            Error::InvalidHistory
        );
    }
}

#[test]
fn strict_quorums_must_overlap_beyond_byzantine_budget() {
    for size in 4..=256 {
        let quorum = (2 * size) / 3 + 1;
        let byzantine_budget = (size - 1) / 3;
        assert!(2 * quorum - size > byzantine_budget);
    }
}

#[test]
fn network_file_is_strict_and_commits_the_ordered_committee() {
    let keys = keys();
    let mut file = b"DIASPORA:WEAVE:NETWORK:V1\0".to_vec();
    file.extend([1; 32]);
    file.extend(GENESIS);
    file.extend(2u64.to_be_bytes());
    file.extend(4u16.to_be_bytes());
    for key in &keys {
        file.extend(key.verifying_key().to_bytes());
    }
    let chain_id = Network::from_bytes(&file).unwrap().chain_id();
    assert_eq!(chain_id, config(&keys, 1).chain_id());
    assert_eq!(
        chain_id,
        <[u8; 32]>::from(<sha2::Sha256 as sha2::Digest>::digest(&file))
    );

    let mut reversed = keys.clone();
    reversed.reverse();
    assert_ne!(chain_id, config(&reversed, 1).chain_id());
    assert!(matches!(
        Network::from_bytes(&file[..file.len() - 1]),
        Err(Error::InvalidNetwork)
    ));
    let duplicate = keys[0].verifying_key().to_bytes();
    let last = file.len() - 32;
    file[last..].copy_from_slice(&duplicate);
    assert!(matches!(
        Network::from_bytes(&file),
        Err(Error::InvalidNetwork)
    ));
    file.push(0);
    assert!(matches!(
        Network::from_bytes(&file),
        Err(Error::InvalidNetwork)
    ));
}
