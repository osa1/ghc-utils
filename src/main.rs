//! Utilities for working on GHC's prof JSON dumps (`+RTS -pj`)
//!
//! For now just compares allocations.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct ProfFile {
    program: String,
    arguments: Vec<String>,
    rts_arguments: Vec<String>,
    end_time: String,
    initial_capabilities: u8,
    total_time: f64,
    total_ticks: u64,
    tick_interval: u64,
    total_alloc: u64,
    cost_centres: Vec<CostCentre>,
    profile: Profile,
}

#[derive(Debug, Deserialize)]
struct CostCentre {
    id: u64,
    label: String,
    module: String,
    src_loc: String,
    is_caf: bool,
}

#[derive(Debug, Deserialize)]
struct Profile {
    id: u64,
    entries: u64,
    alloc: u64,
    ticks: u64,
    children: Vec<Profile>,
}

fn parse_prof_file(path: &str) -> ProfFile {
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

/// Maps cost centres to allocations
fn make_alloc_map(f: &ProfFile) -> HashMap<String, u64> {
    let cost_centre_map = {
        let mut map: HashMap<u64, String> = HashMap::new();
        for cc in &f.cost_centres {
            map.insert(cc.id, format!("{}.{}", cc.module, cc.label));
        }
        map
    };

    let mut alloc_map = HashMap::new();
    add_profile(&mut alloc_map, &f.profile, &cost_centre_map);

    alloc_map
}

fn add_profile(alloc_map: &mut HashMap<String, u64>, p: &Profile, cc_map: &HashMap<u64, String>) {
    alloc_map.insert(cc_map.get(&p.id).unwrap().clone(), p.alloc);
    for p in &p.children {
        add_profile(alloc_map, p, cc_map);
    }
}

fn compare(f1: &str, f2: &str) {
    let allocs1 = make_alloc_map(&parse_prof_file(f1));
    let mut allocs2 = make_alloc_map(&parse_prof_file(f2));

    let mut diffs: Vec<(String, i64)> = vec![];

    for (cc, alloc1) in allocs1.into_iter() {
        let diff = match allocs2.remove(&cc) {
            None => -(alloc1 as i64),
            Some(alloc2) => (alloc2 as i64) - (alloc1 as i64),
        };
        if diff != 0 {
            diffs.push((cc, diff));
        }
    }

    for (cc, alloc2) in allocs2.into_iter() {
        if alloc2 != 0 {
            diffs.push((cc, alloc2 as i64));
        }
    }

    diffs.sort_by_key(|&(_, v)| std::cmp::Reverse(v));

    let mut total = 0;
    for (k, v) in diffs {
        println!("{}: {}", k, v);
        total += v;
    }
    println!();
    println!("TOTAL: {}", total);
}

fn show_allocs(f: &str) {
    let allocs = make_alloc_map(&parse_prof_file(f));
    let mut allocs = allocs.into_iter().collect::<Vec<(String, u64)>>();
    allocs.sort_by_key(|&(_, v)| std::cmp::Reverse(v));

    let total: u64 = allocs.iter().map(|&(_, v)| v).sum();
    let total_f: f64 = total as f64;

    for (cc, alloc) in allocs.iter() {
        if *alloc != 0 {
            println!(
                "{}: {} ({:.2}%)",
                cc,
                alloc,
                ((*alloc as f64) / total_f) * 100.0f64
            );
        }
    }

    println!("TOTAL: {}", total);
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();

    match args.len() {
        3 => {
            compare(&args[1], &args[2]);
        }
        2 => {
            show_allocs(&args[1]);
        }
        _ => {
            println!("USAGE:");
            println!("(1) ghc-prof-compare <file1> <file2> # to compare two files for allocations");
            println!("(2) ghc-prof-compare <file>          # to show allocations, sorted");
            std::process::exit(1);
        }
    }
}
