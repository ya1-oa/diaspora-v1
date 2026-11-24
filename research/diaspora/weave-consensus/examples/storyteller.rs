use diaspora_weave_consensus::tapestry::{Branch, Impact, Kind, Storyteller};
use diaspora_weave_consensus::{
    Certificate, Checkpoint, ForkChoice, Frontier, Network, ValidatedHistory, Vote,
};
use ed25519_dalek::{Signer, SigningKey};
use std::{env, fs};

fn branch(path: &[(u8, u128)], impacts: Vec<Vec<Impact>>) -> Branch {
    let mut parent = [0; 32];
    let frontiers = path
        .iter()
        .map(|&(number, work)| {
            let id = [number; 32];
            let frontier = Frontier { id, parent, work };
            parent = id;
            frontier
        })
        .collect();
    Branch {
        history: ValidatedHistory::from_validated_frontiers([0; 32], frontiers).unwrap(),
        impacts,
    }
}

fn place(x: u16, y: u16, kind: Kind, energy: u16) -> Impact {
    Impact {
        position: (x, y),
        cell: Some((kind, energy)),
    }
}

fn demonstration() -> Storyteller {
    // Fixture keys and caller-validated frontiers for a local demonstration.
    let keys: Vec<_> = (1..=4)
        .map(|id| SigningKey::from_bytes(&[id; 32]))
        .collect();
    let network = Network::new(
        [9; 32],
        [0; 32],
        2,
        keys.iter().map(SigningKey::verifying_key).collect(),
    )
    .unwrap();
    let parent = Checkpoint {
        height: 0,
        id: [0; 32],
    };
    let target = Checkpoint {
        height: 2,
        id: [3; 32],
    };
    let message = network.vote_bytes(parent, target);
    let certificate = Certificate {
        parent,
        target,
        votes: keys
            .iter()
            .take(3)
            .enumerate()
            .map(|(validator, key)| Vote {
                validator,
                signature: key.sign(&message),
            })
            .collect(),
    };
    let mut choice = ForkChoice::new(network);
    let terrain = vec![place(2, 4, Kind::Water, 2), place(3, 3, Kind::Air, 2)];
    let a = branch(
        &[(1, 5), (2, 5)],
        vec![terrain.clone(), vec![place(3, 4, Kind::Seed, 2)]],
    );
    let b = branch(
        &[(1, 5), (3, 9)],
        vec![terrain.clone(), vec![place(3, 5, Kind::Seed, 2)]],
    );
    let c = branch(&[(1, 5), (4, 100)], vec![terrain, vec![]]);
    let mut storyteller = Storyteller::default();
    storyteller
        .observe(&choice, std::slice::from_ref(&a))
        .unwrap();
    let candidates = [a, b];
    storyteller.observe(&choice, &candidates).unwrap();
    let [_, b] = candidates;
    choice.apply(&certificate, Some(&b.history)).unwrap();
    storyteller.observe(&choice, &[b, c]).unwrap();
    storyteller
}

fn main() -> std::io::Result<()> {
    let path = env::args_os()
        .nth(1)
        .unwrap_or_else(|| "tapestry.html".into());
    fs::write(path, demonstration().html())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storyteller_keeps_the_original_world_and_marks_orphaned_lineage() {
        let tapestry = demonstration();
        let frames = tapestry.frames();
        assert_eq!(frames[0].world.cells()[&(3, 4)].kind, Kind::Conduit);
        assert!(frames[1].reorganization);
        assert_eq!(frames[1].orphaned_cells[0].1.origin, [2; 32]);
        assert!(!frames[1].world.cells().contains_key(&(3, 4)));
        assert_eq!(frames[2].selected, Some([3; 32]));
        assert_eq!(frames[2].rejected_by_checkpoint, [[4; 32]]);
        assert!(!frames[2].reorganization);
        assert!(tapestry.html().contains("Orphaned (3, 4)"));
        assert_eq!(tapestry.html(), demonstration().html());
    }

    #[test]
    fn same_visible_kind_and_energy_can_have_different_lineage() {
        let keys = (1..=4)
            .map(|id| SigningKey::from_bytes(&[id; 32]).verifying_key())
            .collect();
        let choice = ForkChoice::new(Network::new([1; 32], [0; 32], 2, keys).unwrap());
        let mut storyteller = Storyteller::default();
        let impact = vec![place(3, 4, Kind::Seed, 3)];
        storyteller
            .observe(&choice, &[branch(&[(1, 1)], vec![impact.clone()])])
            .unwrap();
        storyteller
            .observe(&choice, &[branch(&[(2, 2)], vec![impact])])
            .unwrap();
        let frames = storyteller.frames();
        let a = frames[0].world.cells()[&(3, 4)];
        let b = frames[1].world.cells()[&(3, 4)];
        assert_eq!((a.kind, a.energy), (b.kind, b.energy));
        assert_ne!(a.origin, b.origin);
        assert_eq!(frames[1].orphaned_cells, [((3, 4), a)]);
    }
}
