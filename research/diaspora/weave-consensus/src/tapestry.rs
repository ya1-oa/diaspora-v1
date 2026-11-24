//! Replayable Living Tapestry and local Storyteller observations.
use crate::{Checkpoint, Error, ForkChoice, Id, ValidatedHistory};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub const GRID_SIZE: u16 = 1024;
pub type Position = (u16, u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Earth,
    Water,
    Air,
    Fire,
    Seed,
    Conduit,
    Forge,
    Vault,
}

impl Kind {
    fn glyph(self) -> char {
        match self {
            Self::Earth => 'E',
            Self::Water => 'W',
            Self::Air => 'A',
            Self::Fire => 'F',
            Self::Seed => 'S',
            Self::Conduit => 'C',
            Self::Forge => 'G',
            Self::Vault => 'V',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cell {
    pub kind: Kind,
    pub energy: u16,
    pub origin: Id,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Impact {
    pub position: Position,
    /// None represents Void.
    pub cell: Option<(Kind, u16)>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct World {
    cells: BTreeMap<Position, Cell>,
}

impl World {
    pub fn cells(&self) -> &BTreeMap<Position, Cell> {
        &self.cells
    }

    /// Connected components of Seeds and their mature structures.
    pub fn organisms(&self) -> Vec<Vec<Position>> {
        let mut remaining: BTreeSet<_> = self
            .cells
            .iter()
            .filter(|(_, cell)| {
                matches!(
                    cell.kind,
                    Kind::Seed | Kind::Conduit | Kind::Forge | Kind::Vault
                )
            })
            .map(|(&position, _)| position)
            .collect();
        let mut organisms = Vec::new();
        while let Some(start) = remaining.pop_first() {
            let mut organism = vec![start];
            let mut index = 0;
            while index < organism.len() {
                for neighbor in neighbors(organism[index]) {
                    if remaining.remove(&neighbor) {
                        organism.push(neighbor);
                    }
                }
                index += 1;
            }
            organisms.push(organism);
        }
        organisms
    }

    fn step(&mut self) {
        let snapshot = self.cells.clone();
        for (&position, cell) in &mut self.cells {
            let adjacent = neighbors(position).filter_map(|p| snapshot.get(&p));
            let mut water = false;
            let mut fire = false;
            let mut air = false;
            let mut earth = false;
            for neighbor in adjacent {
                water |= neighbor.kind == Kind::Water;
                fire |= neighbor.kind == Kind::Fire;
                air |= neighbor.kind == Kind::Air;
                earth |= neighbor.kind == Kind::Earth;
            }
            if cell.kind == Kind::Seed {
                cell.energy = if water {
                    cell.energy.saturating_add(2)
                } else {
                    cell.energy.saturating_sub(1)
                };
                if cell.energy >= 4 {
                    cell.kind = match (fire, air, earth) {
                        (true, _, _) => Kind::Forge,
                        (_, true, _) => Kind::Conduit,
                        (_, _, true) => Kind::Vault,
                        _ => Kind::Seed,
                    };
                }
            }
        }
        self.cells.retain(|_, cell| cell.energy != 0);
    }
}

fn neighbors((x, y): Position) -> impl Iterator<Item = Position> {
    (-1i32..=1)
        .flat_map(move |dx| (-1i32..=1).map(move |dy| (dx, dy)))
        .filter(|pair| *pair != (0, 0))
        .filter_map(move |(dx, dy)| {
            let x = i32::from(x) + dx;
            let y = i32::from(y) + dy;
            let range = 0..i32::from(GRID_SIZE);
            (range.contains(&x) && range.contains(&y)).then_some((x as u16, y as u16))
        })
}

/// The ledger adapter must extract ordered impacts from each validated frontier.
/// This observer does not authenticate that mapping.
pub struct Branch {
    pub history: ValidatedHistory,
    pub impacts: Vec<Vec<Impact>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ObserveError {
    InvalidImpacts,
    Consensus(Error),
}

#[derive(Debug)]
pub struct Frame {
    pub checkpoint: Checkpoint,
    pub selected: Option<Id>,
    pub competing: Vec<Id>,
    pub rejected_by_checkpoint: Vec<Id>,
    pub reorganization: bool,
    pub conflicting_certificates: bool,
    /// Positions and original cells whose creating frontier left selected history.
    pub orphaned_cells: Vec<(Position, Cell)>,
    pub world: World,
}

#[derive(Default)]
pub struct Storyteller {
    frames: Vec<Frame>,
    selected_history: Vec<Id>,
    // Check shared frontier data for consistency across observations.
    known_impacts: BTreeMap<Id, Vec<Impact>>,
}

impl Storyteller {
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn observe(
        &mut self,
        choice: &ForkChoice,
        branches: &[Branch],
    ) -> Result<&Frame, ObserveError> {
        let histories: Vec<_> = branches.iter().map(|branch| &branch.history).collect();
        let mut observed = BTreeMap::new();
        for branch in branches {
            if branch.impacts.len() != branch.history.frontiers.len() {
                return Err(ObserveError::InvalidImpacts);
            }
            for (changes, frontier) in branch.impacts.iter().zip(&branch.history.frontiers) {
                if changes.iter().any(|change| {
                    change.position.0 >= GRID_SIZE
                        || change.position.1 >= GRID_SIZE
                        || matches!(change.cell, Some((_, 0)))
                }) || self
                    .known_impacts
                    .get(&frontier.id)
                    .is_some_and(|previous| previous != changes)
                    || observed
                        .insert(frontier.id, changes)
                        .is_some_and(|previous| previous != changes)
                {
                    return Err(ObserveError::InvalidImpacts);
                }
            }
        }
        let checkpoint = choice.finalized();
        let selected = choice.select_iter(histories.iter().copied());
        let conflicting_certificates = matches!(selected, Err(Error::ConflictingFinality));
        let selected = match selected {
            Ok(history) => history,
            Err(Error::ConflictingFinality) => None,
            Err(error) => return Err(ObserveError::Consensus(error)),
        };
        let mut world = self
            .frames
            .last()
            .map_or_else(World::default, |frame| frame.world.clone());
        let mut orphaned_cells = Vec::new();
        let mut reorganization = false;
        if let Some(history) = selected {
            let branch = branches
                .iter()
                .find(|branch| std::ptr::eq(&branch.history, history))
                .expect("selection borrows an observed branch");
            let path: Vec<_> = history
                .frontiers
                .iter()
                .map(|frontier| frontier.id)
                .collect();
            let retained: BTreeSet<_> = path.iter().copied().collect();
            reorganization = !path.starts_with(&self.selected_history);
            orphaned_cells.extend(
                world
                    .cells
                    .iter()
                    .filter(|(_, cell)| !retained.contains(&cell.origin))
                    .map(|(&position, &cell)| (position, cell)),
            );
            world = World::default();
            for (&origin, changes) in path.iter().zip(&branch.impacts) {
                for impact in changes {
                    if let Some((kind, energy)) = impact.cell {
                        world.cells.insert(
                            impact.position,
                            Cell {
                                kind,
                                energy,
                                origin,
                            },
                        );
                    } else {
                        world.cells.remove(&impact.position);
                    }
                }
                world.step();
            }
            self.selected_history = path;
        }
        let competing = histories
            .iter()
            .map(|history| history.tip())
            .filter(|tip| Some(*tip) != selected.map(ValidatedHistory::tip))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let rejected_by_checkpoint = histories
            .iter()
            .filter(|history| {
                history.genesis == choice.network.genesis.id
                    && history
                        .id_at(checkpoint.height)
                        .is_some_and(|id| id != checkpoint.id)
            })
            .map(|history| history.tip())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self.known_impacts.extend(
            observed
                .into_iter()
                .map(|(id, impacts)| (id, impacts.clone())),
        );
        self.frames.push(Frame {
            checkpoint,
            selected: selected.map(ValidatedHistory::tip),
            competing,
            rejected_by_checkpoint,
            reorganization,
            conflicting_certificates,
            orphaned_cells,
            world,
        });
        Ok(self.frames.last().expect("just appended"))
    }

    /// Standalone archive. Covers every observation, including withdrawn states.
    pub fn html(&self) -> String {
        let mut html = String::from(
            r#"<!doctype html><html lang="en"><meta charset="utf-8"><title>Living Tapestry</title>
<style>body{font:16px system-ui;max-width:72rem;margin:3rem auto;padding:0 1rem;background:#141b19;color:#f0f2e8}article{border-left:3px solid #9ccc9b;padding:1rem;margin:1rem 0;background:#202925}code{overflow-wrap:anywhere}li{margin:.3rem 0}.grid{display:grid;grid-template-columns:repeat(12,1.8rem);gap:2px}.grid span{display:grid;place-items:center;aspect-ratio:1;background:#37473e}.grid .void{opacity:.25}.grid .lost{outline:2px solid #f0a093;color:#ffbeb4}</style>
<h1>Living Tapestry</h1><p>Local observations in arrival order. A competing frontier alone does not establish an attack.</p>
<p>E Earth · W Water · A Air · F Fire · S Seed · C Conduit · G Forge · V Vault · × orphaned cell. Map shows the first 12 × 12 cells.</p>"#,
        );
        for (index, frame) in self.frames.iter().enumerate() {
            let event = if frame.conflicting_certificates {
                "Finality conflict"
            } else if frame.reorganization {
                "Provisional reorganization"
            } else {
                "Frontier observation"
            };
            let work = frame.selected.map_or_else(|| "none".into(), hex);
            write!(html, "<article><h2>{} · {event}</h2><p>Selected tip <code>{work}</code><br>Finalized height {} · <code>{}</code></p>",
                index + 1, frame.checkpoint.height, hex(frame.checkpoint.id)).unwrap();
            for tip in &frame.competing {
                let label = if frame.rejected_by_checkpoint.contains(tip) {
                    "Conflicts with checkpoint"
                } else {
                    "Other observed tip"
                };
                write!(html, "<p>{label} <code>{}</code></p>", hex(*tip)).unwrap();
            }
            write!(
                html,
                "<p>{} organisms · {} occupied cells · {} total energy</p>",
                frame.world.organisms().len(),
                frame.world.cells().len(),
                frame
                    .world
                    .cells()
                    .values()
                    .map(|cell| u64::from(cell.energy))
                    .sum::<u64>()
            )
            .unwrap();
            let orphan_positions: BTreeSet<_> =
                frame.orphaned_cells.iter().map(|(p, _)| *p).collect();
            html.push_str("<div class='grid' aria-hidden='true'>");
            for y in 0..12 {
                for x in 0..12 {
                    let position = (x, y);
                    let cell = frame.world.cells.get(&position);
                    let class = if orphan_positions.contains(&position) {
                        "lost"
                    } else if cell.is_none() {
                        "void"
                    } else {
                        ""
                    };
                    let glyph = cell.map_or(if class == "lost" { '×' } else { '·' }, |cell| {
                        cell.kind.glyph()
                    });
                    write!(html, "<span class='{class}'>{glyph}</span>").unwrap();
                }
            }
            html.push_str("</div><details><summary>Cells and provenance</summary><ul>");
            for (&(x, y), cell) in frame.world.cells() {
                write!(
                    html,
                    "<li>({x}, {y}) {:?} · energy {} · <code>{}</code></li>",
                    cell.kind,
                    cell.energy,
                    hex(cell.origin)
                )
                .unwrap();
            }
            for &((x, y), cell) in &frame.orphaned_cells {
                write!(
                    html,
                    "<li>Orphaned ({x}, {y}) {:?} · <code>{}</code></li>",
                    cell.kind,
                    hex(cell.origin)
                )
                .unwrap();
            }
            html.push_str("</ul></details></article>");
        }
        html.push_str("</html>");
        html
    }
}

fn hex(id: Id) -> String {
    let mut output = String::with_capacity(64);
    for byte in id {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}
