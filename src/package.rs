use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::fs;
use std::sync::{Arc, Mutex};

pub struct Package {
    pub name: String,
}

pub struct PackageDownloader {
    pkg_start_idx: usize,
    num_pkgs: usize,
    event_sender: Sender<Event>,
    package_names: Arc<Vec<String>>,
}

impl PackageDownloader {
    pub fn new(
        pkg_start_idx: usize, 
        num_pkgs: usize, 
        event_sender: Sender<Event>,
        package_names: Arc<Vec<String>>,
    ) -> Self {
        Self {
            pkg_start_idx,
            num_pkgs,
            event_sender,
            package_names,
        }
    }

    pub fn run(&self, pkg_checksum: Arc<Mutex<Checksum>>) {
        // Cycle through the package names
        let total_packages = self.package_names.len();
        let cycled_package_names = self.package_names.iter().cycle();
    
        // Generate a set of packages and place them into the event queue
        for i in 0..self.num_pkgs {
            let name = cycled_package_names
                .clone()
                .nth(self.pkg_start_idx + i % total_packages)
                .unwrap();
    
            pkg_checksum
                .lock()
                .unwrap()
                .update(Checksum::with_sha256(&name));
            self.event_sender
                .send(Event::DownloadComplete(Package { name: name.to_string() }))
                .unwrap();
        }
    }
    
    
}
