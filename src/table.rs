use crate::guest::GuestId;

#[derive(Clone, Debug)]
pub struct TableConfig {
    pub capacity: usize,
    pub count: usize,
}

impl TableConfig {
    pub fn new(capacity: usize, count: usize) -> Self {
        Self { capacity, count }
    }

    pub fn total_capacity(&self) -> usize {
        self.capacity * self.count
    }
}

#[derive(Clone, Debug)]
pub struct Table {
    pub id: usize,
    pub capacity: usize,
    pub seats: Vec<Option<GuestId>>,
}

impl Table {
    pub fn new(id: usize, capacity: usize) -> Self {
        Self {
            id,
            capacity,
            seats: vec![None; capacity],
        }
    }

    pub fn num_occupied(&self) -> usize {
        self.seats.iter().filter(|s| s.is_some()).count()
    }

    pub fn free_seats(&self) -> usize {
        self.capacity - self.num_occupied()
    }

    pub fn is_full(&self) -> bool {
        self.free_seats() == 0
    }

    pub fn free_seat_indices(&self) -> Vec<usize> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_none())
            .map(|(i, _)| i)
            .collect()
    }

    pub fn adjacent_seats(&self, seat: usize) -> Vec<usize> {
        if self.capacity <= 2 {
            return (0..self.capacity).filter(|&i| i != seat).collect();
        }
        let left = if seat == 0 { self.capacity - 1 } else { seat - 1 };
        let right = if seat + 1 >= self.capacity { 0 } else { seat + 1 };
        vec![left, right]
    }

    pub fn neighbors(&self, seat: usize) -> Vec<GuestId> {
        self.adjacent_seats(seat)
            .into_iter()
            .filter_map(|i| self.seats[i])
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct SeatingArrangement {
    pub tables: Vec<Table>,
    pub unseated: Vec<GuestId>,
}

impl SeatingArrangement {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            unseated: Vec::new(),
        }
    }

    pub fn is_feasible(&self) -> bool {
        self.unseated.is_empty()
    }

    pub fn seat_count(&self) -> usize {
        self.tables.iter().map(|t| t.num_occupied()).sum()
    }

    pub fn guest_table(&self, id: GuestId) -> Option<(usize, usize)> {
        for (ti, table) in self.tables.iter().enumerate() {
            for (si, seat) in table.seats.iter().enumerate() {
                if *seat == Some(id) {
                    return Some((ti, si));
                }
            }
        }
        None
    }
}
