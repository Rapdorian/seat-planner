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

    // Phase 3: Should Filling
    let unseated: Vec<GuestId> = all_guests.difference(&seated).copied().collect();
    for &guest in &unseated {
        let best = score_tables_for_guest(guest, constraints, &tables);
        if let Some((table_idx, seat_idx)) = best {
            tables[table_idx].seats[seat_idx] = Some(guest);
            seated.insert(guest);
        }
    }

    let remaining: Vec<GuestId> = all_guests.difference(&seated).copied().collect();
    for &guest in &remaining {
        let best = find_any_free_seat(&tables);
        if let Some((table_idx, seat_idx)) = best {
            tables[table_idx].seats[seat_idx] = Some(guest);
            seated.insert(guest);
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

fn score_tables_for_guest(
    guest: GuestId,
    constraints: &ConstraintGraph,
    tables: &[Table],
) -> Option<(usize, usize)> {
    let neighbors = constraints.neighbors(guest);
    let pref_ids: HashSet<GuestId> = neighbors
        .iter()
        .filter(|(_, kind)| *kind == LinkType::Must || *kind == LinkType::Should)
        .map(|(id, _)| *id)
        .collect();

    let mut best: Option<(usize, usize, i32)> = None; // (table_idx, seat_idx, score)

    for (ti, table) in tables.iter().enumerate() {
        for &si in &table.free_seat_indices() {
            let adjacent = table.neighbors(si);
            let score = adjacent.iter().filter(|n| pref_ids.contains(n)).count() as i32;
            // prefer seats with some adjacency over none
            let score = score * 100 + (if adjacent.is_empty() { 0 } else { 1 });
            if best.map_or(true, |(_, _, s)| score > s) {
                best = Some((ti, si, score));
            }
        }
    }

    best.map(|(ti, si, _)| (ti, si))
}

fn find_any_free_seat(tables: &[Table]) -> Option<(usize, usize)> {
    for (ti, table) in tables.iter().enumerate() {
        for &si in &table.free_seat_indices() {
            return Some((ti, si));
        }
    }
    None
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
