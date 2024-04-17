use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::sync::{Arc, Mutex};

pub struct Idea {
    pub name: String,
    pub num_pkg_required: usize,
}

pub struct IdeaGenerator {
    idea_start_idx: usize,
    num_ideas: usize,
    num_students: usize,
    num_pkgs: usize,
    event_sender: Sender<Event>,
    products: Arc<String>,
    customers: Arc<String>
}

impl IdeaGenerator {
    pub fn new(
        idea_start_idx: usize,
        num_ideas: usize,
        num_students: usize,
        num_pkgs: usize,
        event_sender: Sender<Event>,
        products: Arc<String>,
        customers: Arc<String>
    ) -> Self {
        Self {
            idea_start_idx,
            num_ideas,
            num_students,
            num_pkgs,
            event_sender,
            products,
            customers
        }
    }

    // Idea names are generated from cross products between product names and customer names
    pub fn get_next_idea_name(&self, idx: usize) -> String {
        let ideas: Vec<_> = self.products
            .lines()
            .flat_map(|p| self.customers.lines().map(move |c| (p.to_owned(), c.to_owned())))
            .collect();
        let pair = &ideas[idx % ideas.len()];
        format!("{} for {}", pair.0, pair.1)
    }

    pub fn run(&self, idea_checksum: Arc<Mutex<Checksum>>) {
        let pkg_per_idea = self.num_pkgs / self.num_ideas;
        let extra_pkgs = self.num_pkgs % self.num_ideas;

        // Generate a set of new ideas and place them into the event-queue
        // Update the idea checksum with all generated idea names
        for i in 0..self.num_ideas {
            let name = self.get_next_idea_name(self.idea_start_idx + i);
            let extra = (i < extra_pkgs) as usize;
            let num_pkg_required = pkg_per_idea + extra;
            let idea = Idea {
                name,
                num_pkg_required,
            };

            idea_checksum
                .lock()
                .unwrap()
                .update(Checksum::with_sha256(&idea.name));

            self.event_sender.send(Event::NewIdea(idea)).unwrap();
        }

        // Push student termination events into the event queue
        for _ in 0..self.num_students {
            self.event_sender.send(Event::OutOfIdeas).unwrap();
        }
    }
}
