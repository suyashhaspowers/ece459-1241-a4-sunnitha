#![warn(clippy::all)]
use lab4::{
    checksum::Checksum, idea::IdeaGenerator, package::PackageDownloader, student::Student, Event,
};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::fs;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

struct Args {
    pub num_ideas: usize,
    pub num_idea_gen: usize,
    pub num_pkgs: usize,
    pub num_pkg_gen: usize,
    pub num_students: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let num_ideas = args.get(1).map_or(Ok(80), |a| a.parse())?;
    let num_idea_gen = args.get(2).map_or(Ok(2), |a| a.parse())?;
    let num_pkgs = args.get(3).map_or(Ok(4000), |a| a.parse())?;
    let num_pkg_gen = args.get(4).map_or(Ok(6), |a| a.parse())?;
    let num_students = args.get(5).map_or(Ok(6), |a| a.parse())?;
    let args = Args {
        num_ideas,
        num_idea_gen,
        num_pkgs,
        num_pkg_gen,
        num_students,
    };

    hackathon(&args);
    Ok(())
}

fn per_thread_amount(thread_idx: usize, total: usize, threads: usize) -> usize {
    let per_thread = total / threads;
    let extras = total % threads;
    per_thread + (thread_idx < extras) as usize
}

fn hackathon(args: &Args) {
    // Use message-passing channel as event queue
    let (send, recv) = unbounded::<Event>();
    let mut threads = vec![];
    // Checksums of all the generated ideas and packages
    let mut idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut pkg_checksum = Arc::new(Mutex::new(Checksum::default()));
    // Checksums of the ideas and packages used by students to build ideas. Should match the
    // previous checksums.
    let mut student_idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut student_pkg_checksum = Arc::new(Mutex::new(Checksum::default()));

    // Loading in products and customers in one read
    let products = Arc::new(fs::read_to_string("data/ideas-products.txt").expect("file not found"));
    let customers = Arc::new(fs::read_to_string("data/ideas-customers.txt").expect("file not found"));

    // Read packages.txt once here
    let package_names = Arc::new(
        fs::read_to_string("data/packages.txt")
            .expect("file not found")
            .lines()
            .map(|line| line.to_owned())
            .collect::<Vec<String>>(),
    );

    // Create a shared message buffer
    let message_buffer = Arc::new(Mutex::new(Vec::new()));

    // Spawn student threads
    for i in 0..args.num_students {
        let message_buffer_clone = Arc::clone(&message_buffer);
        let mut student = Student::new(i, Sender::clone(&send), Receiver::clone(&recv), message_buffer_clone);
        let student_idea_checksum = Arc::clone(&student_idea_checksum);
        let student_pkg_checksum = Arc::clone(&student_pkg_checksum);
        let thread = spawn(move || student.run(student_idea_checksum, student_pkg_checksum));
        threads.push(thread);
    }

    // Spawn package downloader threads. Packages are distributed evenly across threads.
    let mut start_idx = 0;
    for i in 0..args.num_pkg_gen {
        let num_pkgs = per_thread_amount(i, args.num_pkgs, args.num_pkg_gen);
        let package_names_clone = Arc::clone(&package_names);
        let downloader = PackageDownloader::new(start_idx, num_pkgs, Sender::clone(&send), package_names_clone);
        let pkg_checksum = Arc::clone(&pkg_checksum);
        start_idx += num_pkgs;

        let thread = spawn(move || downloader.run(pkg_checksum));
        threads.push(thread);
    }
    assert_eq!(start_idx, args.num_pkgs);

    // Spawn idea generator threads. Ideas and packages are distributed evenly across threads. In
    // each thread, packages are distributed evenly across ideas.
    let mut start_idx = 0;
    for i in 0..args.num_idea_gen {
        let num_ideas = per_thread_amount(i, args.num_ideas, args.num_idea_gen);
        let num_pkgs = per_thread_amount(i, args.num_pkgs, args.num_idea_gen);
        let num_students = per_thread_amount(i, args.num_students, args.num_idea_gen);
        let products_clone = Arc::clone(&products);
        let customers_clone = Arc::clone(&customers);
        let generator = IdeaGenerator::new(
            start_idx,
            num_ideas,
            num_students,
            num_pkgs,
            Sender::clone(&send),
            products_clone,
            customers_clone
        );
        let idea_checksum = Arc::clone(&idea_checksum);
        start_idx += num_ideas;

        let thread = spawn(move || generator.run(idea_checksum));
        threads.push(thread);
    }
    assert_eq!(start_idx, args.num_ideas);

    // Join all threads
    threads.into_iter().for_each(|t| t.join().unwrap());

    // Print the collected messages
    let messages = message_buffer.lock().unwrap();
    for message in messages.iter() {
        println!("{}", message);
    }

    let idea = Arc::get_mut(&mut idea_checksum).unwrap().get_mut().unwrap();
    let student_idea = Arc::get_mut(&mut student_idea_checksum)
        .unwrap()
        .get_mut()
        .unwrap();
    let pkg = Arc::get_mut(&mut pkg_checksum).unwrap().get_mut().unwrap();
    let student_pkg = Arc::get_mut(&mut student_pkg_checksum)
        .unwrap()
        .get_mut()
        .unwrap();

    println!("\nGlobal checksums:\nIdea Generator: {}\nStudent Idea: {}\nPackage Downloader: {}\nStudent Package: {}", 
        idea, student_idea, pkg, student_pkg);
}
