/*

An example gdb script for this program:

    break GC.c:269
    commands 1
    printf ">>> GC %d\n", major_gc
    continue
    end
    break move
    commands 2
    printf ">>> %p -> %p size: %d\n", from, to, size
    continue
    end

 */

#![feature(or_patterns)]

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::{App, Arg};
use rustyline::error::ReadlineError;
use rustyline::Editor;

static LINE_START: &str = ">>> ";

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Addr(u64);

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

#[derive(Debug)]
struct AddrSize {
    addr: Addr,
    size: u64,
}

#[derive(Debug)]
struct GC {
    major: bool,
    moves_to: HashMap<Addr, AddrSize>,
    moves_from: HashMap<Addr, AddrSize>,
}

impl GC {
    fn new(major: bool) -> GC {
        GC {
            major,
            moves_to: HashMap::new(),
            moves_from: HashMap::new(),
        }
    }
}

fn main() {
    let args = App::new("obj-loc")
        .arg(
            Arg::with_name("gdb-out-file")
                .help("Path to gdb output")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let path = args.value_of("gdb-out-file").unwrap();

    let file = File::open(path).unwrap_or_else(|_| panic!("Unable to open file: {}", path));
    let reader = BufReader::new(file);

    let mut gcs: Vec<GC> = vec![];
    let mut current_gc: Option<GC> = None;

    for line in reader.lines() {
        let line = line.unwrap();

        if line.starts_with(LINE_START) {
            let line = &line[LINE_START.len()..];
            let words: Vec<&str> = line.split_whitespace().collect();
            if words[0] == "GC" {
                if let Some(gc) = current_gc.take() {
                    gcs.push(gc);
                }
                let major = words[1].parse::<u8>().unwrap() == 1;
                current_gc = Some(GC::new(major));
            } else {
                assert!(current_gc.is_some());
                // from '->' to 'size:' size
                let from = Addr(parse_hex_fail(words[0]));
                let to = Addr(parse_hex_fail(words[2]));
                let size = str::parse::<u64>(words[4])
                    .unwrap_or_else(|_| panic!("Unable to parse size: {}", words[4]));
                let current_gc = current_gc.as_mut().unwrap();
                current_gc
                    .moves_to
                    .insert(from, AddrSize { addr: to, size });
                current_gc
                    .moves_from
                    .insert(to, AddrSize { addr: from, size });
            }
        }
    }

    if let Some(gc) = current_gc.take() {
        gcs.push(gc);
    }

    // println!("{:#?}", gcs);

    repl(&gcs);
}

fn parse_hex(s: &str) -> Option<u64> {
    u64::from_str_radix(&s[2..], 16).ok()
}

fn parse_hex_fail(s: &str) -> u64 {
    parse_hex(s).unwrap_or_else(|| panic!("Unable to parse hex: {}", s))
}

fn repl(gcs: &[GC]) {
    let mut rl = Editor::<()>::new();
    loop {
        match rl.readline(">>> ") {
            Ok(line) => match parse_hex(&line) {
                None => {
                    println!("Unable to parse address: {}", line);
                }
                Some(addr) => {
                    print_locs(gcs, addr);
                }
            },
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                break;
            }
            err
            @ Err(ReadlineError::Io(_) | ReadlineError::Utf8Error | ReadlineError::Errno(_)) => {
                println!("Error while reading line: {:?}", err);
                println!("Aborting.");
                break;
            }
        }
    }
}

/// Find all occurences of a given object and print all locations
fn print_locs(gcs: &[GC], addr: u64) {
    for (era, gc) in gcs.iter().enumerate() {
        if let Some(prev_loc) = gc.moves_from.get(&Addr(addr)) {
            print_locs_(gcs, era, prev_loc.addr);
        }
    }
}

/// Print moves of the given object which was moved to its given location at the given era
fn print_locs_(gcs: &[GC], era: usize, addr: Addr) {
    //
    // Backwards search
    //

    let mut first_era = era;
    let mut bwd_moves: Vec<(Addr, Addr)> = vec![];
    let mut current_loc = addr;

    for prev_era in (0..era).rev() {
        let gc = &gcs[prev_era];
        match gc.moves_from.get(&current_loc) {
            None => {
                break;
            }
            Some(prev_addr) => {
                bwd_moves.push((prev_addr.addr, current_loc));
                current_loc = prev_addr.addr;
                first_era = prev_era;
            }
        }
    }

    //
    // Forwards search
    //

    let mut fwd_moves: Vec<(Addr, Addr)> = vec![];
    let mut current_loc = addr;

    for gc in &gcs[era..] {
        match gc.moves_to.get(&current_loc) {
            None => {
                break;
            }
            Some(next_addr) => {
                fwd_moves.push((current_loc, next_addr.addr));
                current_loc = next_addr.addr;
            }
        }
    }

    // Print locations
    for (i, bwd_move) in bwd_moves.iter().rev().enumerate() {
        println!("{}: {:#?} -> {:#?}", first_era + i, bwd_move.0, bwd_move.1);
    }

    for (i, fwd_move) in fwd_moves.iter().enumerate() {
        println!("{}: {:#?} -> {:#?}", era + i, fwd_move.0, fwd_move.1);
    }
}
