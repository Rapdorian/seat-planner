use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::guest::GuestId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LinkType {
    Must,
    Should,
    Could,
    Wont,
}

impl LinkType {
    pub fn variants() -> &'static [LinkType] {
        &[LinkType::Must, LinkType::Should, LinkType::Could, LinkType::Wont]
    }

    pub fn label(&self) -> &str {
        match self {
            LinkType::Must => "Must",
            LinkType::Should => "Should",
            LinkType::Could => "Could",
            LinkType::Wont => "Wont",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub a: GuestId,
    pub b: GuestId,
    pub kind: LinkType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstraintGraph {
    edges: HashMap<GuestId, Vec<(GuestId, LinkType)>>,
    constraints: Vec<Constraint>,
}

impl ConstraintGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    pub fn add(&mut self, a: GuestId, b: GuestId, kind: LinkType) {
        if a == b {
            return;
        }
        // remove any existing constraint between these two
        self.constraints.retain(|c| !(c.a == a && c.b == b || c.a == b && c.b == a));
        self.edges.entry(a).or_default().retain(|(other, _)| *other != b);
        self.edges.entry(b).or_default().retain(|(other, _)| *other != a);

        self.constraints.push(Constraint { a, b, kind });
        self.edges.entry(a).or_default().push((b, kind));
        self.edges.entry(b).or_default().push((a, kind));
    }

    pub fn remove(&mut self, a: GuestId, b: GuestId) {
        self.constraints.retain(|c| !(c.a == a && c.b == b || c.a == b && c.b == a));
        self.edges.entry(a).or_default().retain(|(other, _)| *other != b);
        self.edges.entry(b).or_default().retain(|(other, _)| *other != a);
    }

    pub fn remove_guest(&mut self, id: GuestId) {
        self.constraints.retain(|c| c.a != id && c.b != id);
        self.edges.remove(&id);
        for edges in self.edges.values_mut() {
            edges.retain(|(other, _)| *other != id);
        }
    }

    pub fn get(&self, a: GuestId, b: GuestId) -> Option<LinkType> {
        self.edges.get(&a)?.iter().find(|(other, _)| *other == b).map(|(_, k)| *k)
    }

    pub fn neighbors(&self, id: GuestId) -> Vec<(GuestId, LinkType)> {
        self.edges.get(&id).cloned().unwrap_or_default()
    }

    pub fn all_constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    pub fn rebuild_edges(&mut self) {
        self.edges.clear();
        for c in &self.constraints {
            self.edges.entry(c.a).or_default().push((c.b, c.kind));
            self.edges.entry(c.b).or_default().push((c.a, c.kind));
        }
    }

    pub fn must_components(&self) -> Vec<Vec<GuestId>> {
        let mut visited = std::collections::HashSet::new();
        let mut components = Vec::new();

        for &guest in self.edges.keys() {
            if visited.contains(&guest) {
                continue;
            }
            let mut component = Vec::new();
            let mut stack = vec![guest];
            while let Some(current) = stack.pop() {
                if !visited.insert(current) {
                    continue;
                }
                component.push(current);
                for (neighbor, kind) in self.edges.get(&current).into_iter().flatten() {
                    if *kind == LinkType::Must && !visited.contains(neighbor) {
                        stack.push(*neighbor);
                    }
                }
            }
            if !component.is_empty() {
                components.push(component);
            }
        }

        components
    }
}
