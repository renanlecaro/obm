use std::io::{self};
use std::thread;
use std::thread::{available_parallelism, JoinHandle};
use std::time::Instant;

use crossterm::ExecutableCommand;

use obm::*;

fn main() {
    let (width, height) = (80, 40);

    let source: String = io::stdin()
        .lines()
        .map(|l| l.unwrap_or_else(|_| String::new()))
        .collect::<Vec<String>>()
        .join("\n");

    let threads_counts = available_parallelism().unwrap().get();

    let (mut best_world, story) = Individual::from_string(&source, width as i32, height as i32);

    // Mostly for the first run
    best_world.improve();

    // let mut best_score = best_world.fitness(&factors);
    let mut previous_best_world = best_world.clone();
    let max_stalled_runs = 20;

    let mut best_score = best_world.score().0;
    let mut runs_with_no_improvement = 0;
    while runs_with_no_improvement < max_stalled_runs {
        let _start_all = Instant::now();
        runs_with_no_improvement += 1;
        let handles: Vec<JoinHandle<Individual>> = (0..threads_counts)
            .into_iter()
            .map(|_index| {
                let mut clone = best_world.clone();
                thread::spawn(|| {
                    clone.mutate();
                    clone
                })
            })
            .collect();

        for handle in handles {
            if let clone = handle.join().unwrap() {
                let score = clone.score().0;
                if score < best_score {
                    best_score = score;
                    runs_with_no_improvement = 0;
                    previous_best_world = best_world.clone();
                    best_world = clone;
                }
            }
        }
    }

    for step in story {
        println!("{}", best_world.to_string(&step));
        println!("{}", step.md);
    }
}
