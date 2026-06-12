use std::collections::HashSet;
use std::fmt;

use crate::constraint::{ConstraintGraph, LinkType};
use crate::guest::GuestId;
use crate::table::{SeatingArrangement, Table};

#[derive(Clone, Debug)]
pub enum SolveError {
    NoTables,
    InsufficientCapacity { total: usize, needed: usize },
    UnsatisfiableMust { component: Vec<GuestId>, max_table: usize },
    UnsatisfiableWont { a: GuestId, b: GuestId },
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::NoTables => write!(f, "No tables defined"),
            SolveError::InsufficientCapacity { total, needed } => {
                write!(f, "Total capacity ({}) is less than guest count ({})", total, needed)
            }
            SolveError::UnsatisfiableMust { component, max_table } => {
                write!(f, "Must-component of size {} exceeds largest table capacity ({})", component.len(), max_table)
            }
            SolveError::UnsatisfiableWont { a, b } => {
                write!(f, "Cannot satisfy Wont constraint between guest {} and {}", a, b)
            }
        }
    }
}

pub type SolveResult<T> = Result<T, SolveError>;

pub fn solve(
    guest_ids: &[GuestId],
    constraints: &ConstraintGraph,
    table_capacities: &[usize],
) -> SolveResult<SeatingArrangement> {
    if table_capacities.is_empty() {
        return Err(SolveError::NoTables);
    }

    let total_capacity: usize = table_capacities.iter().sum();
    if total_capacity < guest_ids.len() {
        return Err(SolveError::InsufficientCapacity {
            total: total_capacity,
            needed: guest_ids.len(),
        });
    }

    let max_capacity = *table_capacities.iter().max().unwrap();

    // Check Must components fit
    for component in constraints.must_components() {
        if component.len() > max_capacity {
            return Err(SolveError::UnsatisfiableMust {
                component,
                max_table: max_capacity,
            });
        }
    }

    let mut tables: Vec<Table> = table_capacities
        .iter()
        .enumerate()
        .map(|(i, &cap)| Table::new(i, cap))
        .collect();

    let mut seated: HashSet<GuestId> = HashSet::new();
    let all_guests: HashSet<GuestId> = guest_ids.iter().copied().collect();

    // Phase 1: Must Clustering
    for component in constraints.must_components() {
        place_component(&component, &mut tables, &mut seated);
    }

    // Phase 2: Wont Separation — check and fix
    fix_wont_violations(constraints, &mut tables, &mut seated)?;

    // Phase 3: Multi-round, seat with ≥2 Must/Should neighbors
    // Sort by connection count so high-connection guests seed tables first,
    // and repeat rounds so later-placed connections unlock more placements.
    loop {
        let unseated: Vec<GuestId> = all_guests.difference(&seated).copied().collect();
        if unseated.is_empty() {
            break;
        }
        let mut candidates = unseated;
        candidates.sort_by(|a, b| count_connections(*b, constraints, &[LinkType::Must, LinkType::Should])
            .cmp(&count_connections(*a, constraints, &[LinkType::Must, LinkType::Should])));
        let mut placed = false;
        for &guest in &candidates {
            if let Some(pos) = find_seat_min_adjacent(guest, constraints, &tables, 2, &[LinkType::Must, LinkType::Should]) {
                seat_guest(guest, pos, &mut tables, &mut seated);
                placed = true;
            }
        }
        if !placed {
            break;
        }
    }

    // Phase 4: Multi-round, try ≥2 Must/Should/Could neighbors
    loop {
        let remaining: Vec<GuestId> = all_guests.difference(&seated).copied().collect();
        if remaining.is_empty() {
            break;
        }
        let mut candidates = remaining;
        candidates.sort_by(|a, b| count_connections(*b, constraints, &[LinkType::Must, LinkType::Should, LinkType::Could])
            .cmp(&count_connections(*a, constraints, &[LinkType::Must, LinkType::Should, LinkType::Could])));
        let mut placed = false;
        for &guest in &candidates {
            if let Some(pos) = find_seat_min_adjacent(guest, constraints, &tables, 2, &[LinkType::Must, LinkType::Should, LinkType::Could]) {
                seat_guest(guest, pos, &mut tables, &mut seated);
                placed = true;
            }
        }
        if !placed {
            break;
        }
    }

    // Phase 5: Best effort for anyone still left
    let remaining: Vec<GuestId> = all_guests.difference(&seated).copied().collect();
    for &guest in &remaining {
        if let Some(pos) = find_best_available_seat(guest, constraints, &tables) {
            seat_guest(guest, pos, &mut tables, &mut seated);
        }
    }

    let final_unseated: Vec<GuestId> = all_guests.difference(&seated).copied().collect();
    Ok(SeatingArrangement {
        tables,
        unseated: final_unseated,
    })
}

fn place_component(component: &[GuestId], tables: &mut [Table], seated: &mut HashSet<GuestId>) {
    if component.is_empty() {
        return;
    }

    // find a table with enough room
    for table in tables.iter_mut() {
        if table.free_seats() >= component.len() {
            for &guest in component {
                let free = table.free_seat_indices();
                if let Some(&seat) = free.first() {
                    table.seats[seat] = Some(guest);
                    seated.insert(guest);
                }
            }
            return;
        }
    }

    // no single table fits — spread across tables, keep members adjacent within each table
    let mut remaining: Vec<GuestId> = component.to_vec();
    for table in tables.iter_mut() {
        if remaining.is_empty() {
            break;
        }
        while !remaining.is_empty() && !table.is_full() {
            let guest = remaining.remove(0);
            let free = table.free_seat_indices();
            // try to place adjacent to already-placed component members
            let mut best = None;
            for &seat in &free {
                let neighbors = table.neighbors(seat);
                let same_component = neighbors.iter().filter(|n| component.contains(n)).count();
                if best.map_or(true, |(_, count)| same_component > count) {
                    best = Some((seat, same_component));
                }
            }
            if let Some((seat, _)) = best {
                table.seats[seat] = Some(guest);
                seated.insert(guest);
            }
        }
    }
}

fn fix_wont_violations(
    constraints: &ConstraintGraph,
    tables: &mut [Table],
    _seated: &mut HashSet<GuestId>,
) -> SolveResult<()> {
    for constraint in constraints.all_constraints() {
        if constraint.kind != LinkType::Wont {
            continue;
        }
        let a = constraint.a;
        let b = constraint.b;

        let a_pos = find_guest(a, tables);
        let b_pos = find_guest(b, tables);

        if let (Some((ti_a, _)), Some((ti_b, _))) = (a_pos, b_pos) {
            if ti_a != ti_b {
                continue; // already on different tables
            }
            // same table — need to move one to a different table
            // try moving b to another table
            let moved = (0..tables.len())
                .filter(|&i| i != ti_a && tables[i].free_seats() > 0)
                .find(|&i| {
                    // check no wont violations on target table
                    let target_guests: Vec<GuestId> = tables[i]
                        .seats
                        .iter()
                        .filter_map(|s| *s)
                        .collect();
                    !target_guests
                        .iter()
                        .any(|&tg| constraints.get(b, tg) == Some(LinkType::Wont))
                });

            if let Some(target) = moved {
                // remove b from current seat
                let (_, si_b) = b_pos.unwrap();
                tables[ti_a].seats[si_b] = None;
                // place b in target
                let free = tables[target].free_seat_indices();
                if let Some(&seat) = free.first() {
                    tables[target].seats[seat] = Some(b);
                }
            } else {
                return Err(SolveError::UnsatisfiableWont { a, b });
            }
        }
    }
    Ok(())
}

fn count_connections(guest: GuestId, constraints: &ConstraintGraph, kinds: &[LinkType]) -> usize {
    constraints.neighbors(guest).iter()
        .filter(|(_, k)| kinds.contains(k))
        .count()
}

fn seat_guest(guest: GuestId, pos: (usize, usize), tables: &mut [Table], seated: &mut HashSet<GuestId>) {
    tables[pos.0].seats[pos.1] = Some(guest);
    seated.insert(guest);
}

fn find_seat_min_adjacent(
    guest: GuestId,
    constraints: &ConstraintGraph,
    tables: &[Table],
    min_strong: usize,
    kinds: &[LinkType],
) -> Option<(usize, usize)> {
    let strong_set: HashSet<GuestId> = constraints
        .neighbors(guest)
        .into_iter()
        .filter(|(_, kind)| kinds.contains(kind))
        .map(|(id, _)| id)
        .collect();

    if strong_set.len() < min_strong {
        return None;
    }

    let mut best: Option<(usize, usize, usize)> = None;
    for (ti, table) in tables.iter().enumerate() {
        for &si in &table.free_seat_indices() {
            let adjacent = table.neighbors(si);
            let count = adjacent.iter().filter(|n| strong_set.contains(n)).count();
            if count >= min_strong {
                if best.map_or(true, |(_, _, s)| count > s) {
                    best = Some((ti, si, count));
                }
            }
        }
    }
    best.map(|(ti, si, _)| (ti, si))
}

fn find_best_available_seat(
    guest: GuestId,
    constraints: &ConstraintGraph,
    tables: &[Table],
) -> Option<(usize, usize)> {
    let conn_set: HashSet<GuestId> = constraints
        .neighbors(guest)
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    let mut best: Option<(usize, usize, usize)> = None;
    for (ti, table) in tables.iter().enumerate() {
        for &si in &table.free_seat_indices() {
            let adjacent = table.neighbors(si);
            let count = adjacent.iter().filter(|n| conn_set.contains(n)).count();
            if best.map_or(true, |(_, _, s)| count > s) {
                best = Some((ti, si, count));
            }
        }
    }
    best.map(|(ti, si, _)| (ti, si))
}

fn find_guest(id: GuestId, tables: &[Table]) -> Option<(usize, usize)> {
    for (ti, table) in tables.iter().enumerate() {
        for (si, seat) in table.seats.iter().enumerate() {
            if *seat == Some(id) {
                return Some((ti, si));
            }
        }
    }
    None
}
