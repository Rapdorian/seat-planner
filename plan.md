# Seat Planner — Implementation Plan

## Overview

A Rust + egui desktop/WASM app for managing guest lists and solving seating arrangements with
constraints. The solver assigns guests to tables (mixed sizes) respecting Must/Should/Could/Wont
relationships, targeting 2-3 strong connections per guest.

---

## Architecture

### Module Layout

```
src/
├── main.rs              # native entry (unchanged)
├── lib.rs               # App shell, eframe::App impl, WASM entry
├── guest.rs             # GuestId, Guest, GuestList
├── constraint.rs        # LinkType enum, Constraint, ConstraintGraph
├── table.rs             # TableConfig, Table, SeatingArrangement
├── solver.rs            # constraint satisfaction solver
└── ui.rs                # all egui panels
```

### Data Structures

```rust
// guest.rs
pub type GuestId = usize;

pub struct Guest {
    pub id: GuestId,
    pub name: String,
}

pub struct GuestList {
    pub guests: Vec<Guest>,
    next_id: GuestId,
}
```

```rust
// constraint.rs
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Must,    // must sit together (adjacent)
    Should,  // should sit together (adjacent if possible)
    Could,   // no preference
    Wont,    // must NOT sit together
}

pub struct Constraint {
    pub a: GuestId,
    pub b: GuestId,
    pub kind: LinkType,
}

pub struct ConstraintGraph {
    // adjacency: GuestId -> Vec<(GuestId, LinkType)>
    edges: HashMap<GuestId, Vec<(GuestId, LinkType)>>,
}
```

```rust
// table.rs
pub struct TableConfig {
    pub capacity: usize,
    pub count: usize,         // how many tables of this size
}

pub struct Table {
    pub id: usize,
    pub capacity: usize,
    pub seats: Vec<Option<GuestId>>,   // ordered seats around the table
}

pub struct SeatingArrangement {
    pub tables: Vec<Table>,
    pub unseated: Vec<GuestId>,
}
```

---

## Solver Algorithm

### Phase 1: Must Clustering
1. Build connected components of the **Must** subgraph (undirected).
2. For each component: if component size > largest table capacity, split into subgroups.
3. Assign each Must-component to a table block. Place members in adjacent seats.

### Phase 2: Wont Separation
4. For each **Wont** pair: if already at the same table, swap one member to a different table
   (preferring one where they have zero Wont conflicts).

### Phase 3: Should Filling
5. Collect unseated guests. For each:
   - Score each table's available seats by counting how many **Must** or **Should** neighbors
     are already at that table.
   - Assign to highest-scoring table, adjacent to the most preferred neighbors.
   - Target: every guest has 2-3 Must/Should neighbors.

### Phase 4: Remaining
6. **Could** and unconstrained guests fill any remaining seats arbitrarily.

### Constraint Priority
- **Must**: hard constraint — failure if cannot seat together
- **Wont**: hard constraint — failure if seated at same table (or adjacent)
- **Should**: soft constraint — optimizer target (2-3 per guest)
- **Could**: no-op

---

## UI Panels

### Guest List Panel
- Text input + "Add Guest" button
- Scrollable list of guests with "Remove" button each
- Shows total guest count

### Constraints Panel
- Two dropdowns selecting guests A and B
- Radio/selection for link type (Must / Should / Could / Wont)
- "Add Constraint" button
- Scrollable list of existing constraints with "Remove"

### Tables Panel
- "Add Table" button with capacity spinner (default 8)
- Shows list of all tables and their capacities
- Total seat count vs guest count indicator

### Solver Panel
- "Solve" button (runs constraint solver)
- Shows error if:
  - No tables defined
  - Total capacity < guest count
  - Unsatisfiable Must constraints

### Seating View Panel
- Renders each table as a circle of labeled seats
- Shows which guest is assigned to each seat
- Color-coded seat borders by constraint satisfaction:
  - Green = adjacent to 2+ Must/Should
  - Yellow = adjacent to 1
  - Red = adjacent to 0
- Lists unseated guests if any

---

## Data Flow

```
GuestList --add/remove--> ConstraintGraph --solve--> SeatingArrangement
                          ^                          |
                          |  (reads constraints)     |
                          +------- tables -----------+
```

The solver takes immutable references to `GuestList`, `ConstraintGraph`, and `Vec<TableConfig>`,
and produces a new `SeatingArrangement`.

---

## Implementation Order

1. **guest.rs** — Guest, GuestList, GuestId
2. **constraint.rs** — LinkType, Constraint, ConstraintGraph
3. **table.rs** — TableConfig, Table, SeatingArrangement
4. **solver.rs** — Must clustering, Wont separation, Should scoring, greedy fill
5. **ui.rs** — all egui panels wired into `SeatPlannerApp`
6. **lib.rs** — integrate modules, add state fields to `SeatPlannerApp`

---

## Future Considerations

- **Load/save**: serialize guest list + constraints + table config to JSON
- **Undo/redo**: stack of app state snapshots
- **Drag & drop**: manual reseating in the seating view
- **Export**: printable seating chart (PDF/image via canvas)
