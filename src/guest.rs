pub type GuestId = usize;

#[derive(Clone, Debug)]
pub struct Guest {
    pub id: GuestId,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct GuestList {
    pub guests: Vec<Guest>,
    next_id: GuestId,
    pub filter: String,
}

impl GuestList {
    pub fn new() -> Self {
        Self {
            guests: Vec::new(),
            next_id: 0,
            filter: String::new(),
        }
    }

    pub fn add(&mut self, name: String) {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        if self.guests.iter().any(|g| g.name == trimmed) {
            return;
        }
        self.guests.push(Guest {
            id: self.next_id,
            name: trimmed,
        });
        self.next_id += 1;
    }

    pub fn remove(&mut self, id: GuestId) {
        self.guests.retain(|g| g.id != id);
    }

    pub fn len(&self) -> usize {
        self.guests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.guests.is_empty()
    }

    pub fn get(&self, id: GuestId) -> Option<&Guest> {
        self.guests.iter().find(|g| g.id == id)
    }

    pub fn filtered(&self) -> Vec<&Guest> {
        if self.filter.is_empty() {
            self.guests.iter().collect()
        } else {
            let lower = self.filter.to_lowercase();
            self.guests
                .iter()
                .filter(|g| g.name.to_lowercase().contains(&lower))
                .collect()
        }
    }
}
