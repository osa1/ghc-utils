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
    break Evac.c:148
    commands 3
    printf ">>> %p -> %p size: %d\n", from, to, size
    continue
    end

 */

#![feature(or_patterns)]

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};

use ansi_term::Color;
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

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
struct Moves {
    /// Object first appears in this era.
    era: u64,
    /// All the moves of the objects. First move is in `era`.
    moves: Vec<Addr>,
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

    let gcs = parse(reader);
    repl(&gcs);
}

fn parse<B: BufRead>(reader: B) -> Vec<GC> {
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

    gcs
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
                    for moves in find_moves(gcs, addr) {
                        let mut era = moves.era;
                        for move_ in moves.moves {
                            if move_.0 == addr {
                                println!(
                                    "{}: {}{:#?}{}",
                                    era,
                                    Color::Blue.prefix(),
                                    move_,
                                    Color::Blue.suffix()
                                );
                            } else {
                                println!("{}: {:#?}", era, move_);
                            };

                            era += 1;
                        }
                        println!();
                    }
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

fn find_moves(gcs: &[GC], addr: u64) -> Vec<Moves> {
    let addr = Addr(addr);
    let mut ret = vec![];

    let mut skip_next_fwd = false;
    for (era, gc) in gcs.iter().enumerate() {
        if let Some(prev_addr) = gc.moves_from.get(&addr) {
            ret.push(find_locs(gcs, era, prev_addr.addr));
            skip_next_fwd = true;
            continue;
        }

        if skip_next_fwd {
            skip_next_fwd = false;
            continue;
        }

        skip_next_fwd = false;

        if gc.moves_to.get(&addr).is_some() {
            ret.push(find_locs(gcs, era, addr));
        }
    }

    ret
}

/// Find moves of the given object which was moved to its given location at the given era
fn find_locs(gcs: &[GC], era: usize, addr: Addr) -> Moves {
    // println!("find_locs: era={}, addr={:#?}", era, addr);

    //
    // Backwards search
    //

    let mut first_era = era;
    let mut bwd_moves: Vec<Addr> = vec![];
    let mut current_loc = addr;

    for prev_era in (0..era).rev() {
        let gc = &gcs[prev_era];
        match gc.moves_from.get(&current_loc) {
            None => {
                break;
            }
            Some(prev_addr) => {
                bwd_moves.push(prev_addr.addr);
                current_loc = prev_addr.addr;
                first_era = prev_era;
            }
        }
    }

    //
    // Forwards search
    //

    let mut fwd_moves: Vec<Addr> = vec![];
    let mut current_loc = addr;

    for gc in &gcs[era..] {
        match gc.moves_to.get(&current_loc) {
            None => {
                break;
            }
            Some(next_addr) => {
                fwd_moves.push(next_addr.addr);
                current_loc = next_addr.addr;
            }
        }
    }

    bwd_moves.reverse();
    bwd_moves.push(addr);
    bwd_moves.extend_from_slice(&fwd_moves);

    Moves {
        era: first_era as u64,
        moves: bwd_moves,
    }
}

#[test]
fn parse_test() {
    let input = "\
        >>> GC 1\n\
        >>> 0x123 -> 0x124 size: 1\n\
    ";

    let gcs = parse(input.as_bytes());
    assert_eq!(gcs.len(), 1);
    assert_eq!(
        gcs[0].moves_to.get(&Addr(0x123)),
        Some(&AddrSize {
            addr: Addr(0x124),
            size: 1
        })
    );
    assert_eq!(
        gcs[0].moves_from.get(&Addr(0x124)),
        Some(&AddrSize {
            addr: Addr(0x123),
            size: 1
        })
    );

    let input = "\
        >>> GC 1\n\
        >>> 0x123 -> 0x124 size: 1\n\
        >>> GC 2\n\
        >>> 0x124 -> 0x125 size: 2\n\
        >>> 0x100 -> 0x101 size: 3\n\
    ";

    let gcs = parse(input.as_bytes());
    assert_eq!(gcs.len(), 2);
    assert_eq!(
        gcs[1].moves_to.get(&Addr(0x124)),
        Some(&AddrSize {
            addr: Addr(0x125),
            size: 2
        })
    );
}

#[test]
fn find_moves_test() {
    let input = "\
        >>> GC 1\n\
        >>> 0x123 -> 0x124 size: 1\n\
        >>> GC 2\n\
        >>> 0x124 -> 0x125 size: 2\n\
        >>> 0x100 -> 0x101 size: 3\n\
    ";

    let gcs = parse(input.as_bytes());

    //
    // Test fwd search
    //

    assert_eq!(
        find_moves(&gcs, 0x123),
        vec![Moves {
            era: 0,
            moves: vec![Addr(0x123), Addr(0x124), Addr(0x125)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x100),
        vec![Moves {
            era: 1,
            moves: vec![Addr(0x100), Addr(0x101)],
        }]
    );

    //
    // Test bwd search
    //

    assert_eq!(
        find_moves(&gcs, 0x101),
        vec![Moves {
            era: 1,
            moves: vec![Addr(0x100), Addr(0x101)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x125),
        vec![Moves {
            era: 0,
            moves: vec![Addr(0x123), Addr(0x124), Addr(0x125)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x124),
        vec![Moves {
            era: 0,
            moves: vec![Addr(0x123), Addr(0x124), Addr(0x125)],
        }]
    );
}

#[test]
fn complicated_test() {
    // An interesting case that can legitemately happen in compacting GC: We move x to y, and z to
    // x, in the same GC. Make sure we handle this correctly.

    let input = "\
        >>> GC 1\n\
        >>> 0x124 -> 0x125 size: 2\n\
        >>> 0x125 -> 0x126 size: 2\n\
    ";

    let gcs = parse(input.as_bytes());

    assert_eq!(
        find_moves(&gcs, 0x124),
        vec![Moves {
            era: 0,
            moves: vec![Addr(0x124), Addr(0x125)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x125),
        vec![Moves {
            era: 0,
            moves: vec![Addr(0x124), Addr(0x125)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x126),
        vec![Moves {
            era: 0,
            moves: vec![Addr(0x125), Addr(0x126)],
        }]
    );
}
