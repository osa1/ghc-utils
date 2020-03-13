// See README for example gdb commands to generate logs for this program.

// TODO: This ccurrently assumes if an object is not moved in a GC it dies, which is not correct.
// E.g. an object in the oldest generation is not moved in minor GCs.

// TODO: The gdb script below does not print x->x when compacting GC skips an object because it's
// new location is the same as the current one.

// NOTE: All indices below are 0-based, but when printing GC indices we print 1-based, so the first
// GC is printed as "1".

#![feature(or_patterns)]

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};

use ansi_term::{Color, Style};
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
    /// Is this a major GC?
    major: bool,

    /// All moves in this GC. Note that in compacting GC we can see moves that are normally invalid
    /// in two-space copying GC, e.g. `y -> z; x -> y`.
    moves_fwd: HashMap<Addr, AddrSize>,

    /// Reverse of `moves_fwd`. It's possible to revert `moves_fwd` because in a single GC we can't
    /// move two objects to the same location.
    /// E.g. we'll never see something like `x -> y; z -> y`.
    ///
    /// In other words, in a GC, an object is moved at most once, and a location gets at most one
    /// object.
    moves_bwd: HashMap<Addr, AddrSize>,
}

impl GC {
    fn new(major: bool) -> GC {
        GC {
            major,
            moves_fwd: HashMap::new(),
            moves_bwd: HashMap::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Moves {
    /// The location we searched for.
    loc: Addr,
    /// The first GC in which we've made a move `x -> y`, and the moves `y -> z`, ... eventually
    /// reached `loc`.
    first_move: usize,
    /// All the moves of the objects. First move happens at `gc`th GC.
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

                insert_new(&mut current_gc.moves_fwd, from, AddrSize { addr: to, size });
                insert_new(&mut current_gc.moves_bwd, to, AddrSize { addr: from, size });
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

fn insert_new<K, V>(m: &mut HashMap<K, V>, k: K, v: V)
where
    K: Eq + std::hash::Hash,
{
    let ret = m.insert(k, v);
    assert!(ret.is_none());
}

fn repl(gcs: &[GC]) {
    let mut last_major_gc = 0;
    for (gc_idx, gc) in gcs.iter().enumerate().rev() {
        if gc.major {
            last_major_gc = gc_idx + 1;
            break;
        }
    }

    println!("Total GCs: {}", gcs.len());
    println!("Last major GC: {}", last_major_gc);
    println!("`N: addr` means by the beginning on Nth GC the object lived at addr");

    let mut rl = Editor::<()>::new();

    let bold = Style::new().bold();
    let blue = Color::Blue;

    loop {
        match rl.readline(">>> ") {
            Ok(line) if line.trim().is_empty() => {}
            Ok(line) => match parse_hex(&line) {
                None => {
                    println!("Unable to parse address: {}", line);
                }
                Some(addr) => {
                    for moves in find_moves(gcs, addr) {
                        // Nth GC, 0-based
                        let mut gc_n = moves.first_move;
                        for move_ in moves.moves {
                            // When the object lives at the end of the run gc_n will be gcs.len(),
                            // handle that case
                            let highlight_gc = gc_n < gcs.len() && gcs[gc_n].major;
                            let highlight_addr = move_.0 == addr;

                            if highlight_gc {
                                print!("{}", bold.paint(format!("{}: ", gc_n + 1)));
                            } else {
                                print!("{}: ", gc_n + 1);
                            }

                            if highlight_addr {
                                println!("{}", blue.paint(format!("{:#?}", move_)));
                            } else {
                                println!("{:#?}", move_);
                            }

                            gc_n += 1;
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

/// Find all moves of an object.
fn find_moves(gcs: &[GC], addr: u64) -> Vec<Moves> {
    let addr = Addr(addr);
    let mut ret = vec![];

    // Searching for 'addr'. Two cases:
    //
    // - We see `addr -> y`:
    //   - Follow 'y' starting from next GC.
    //   - Reverse follow 'addr' starting from previous GC.
    //
    // - We see `y -> addr`:
    //   - Follow 'addr' starting from next GC.
    //   - Reverse follow 'y' starting from previous GC.
    //
    // First case happens when `addr` is allocated by a mutator rather than GC.

    // In the second case we will follow 'addr' in the next GC so we should skip the first case in
    // the next iteration:
    let mut skip_first_case = false;

    for (gc_n, gc) in gcs.iter().enumerate() {
        if !skip_first_case {
            // First case, 'x -> y', `next_addr` is 'y'
            if let Some(next_addr) = gc.moves_fwd.get(&addr) {
                let fwd_moves = follow_fwd(&gcs[gc_n + 1..], next_addr.addr);
                let mut bwd_moves = follow_bwd(&gcs[0..gc_n], addr);
                let first_move = gc_n - bwd_moves.len();
                bwd_moves.reverse();
                bwd_moves.push(addr);
                bwd_moves.push(next_addr.addr);
                bwd_moves.extend_from_slice(&fwd_moves);
                ret.push(Moves {
                    loc: addr,
                    first_move,
                    moves: bwd_moves,
                });
            }
        }

        skip_first_case = false;

        // Second case, 'y -> x', `prev_addr` is 'y'
        if let Some(prev_addr) = gc.moves_bwd.get(&addr) {
            let fwd_moves = follow_fwd(&gcs[gc_n + 1..], addr);
            let mut bwd_moves = follow_bwd(&gcs[0..gc_n], prev_addr.addr);
            let first_move = gc_n - bwd_moves.len();
            bwd_moves.reverse();
            bwd_moves.push(prev_addr.addr);
            bwd_moves.push(addr);
            bwd_moves.extend_from_slice(&fwd_moves);
            ret.push(Moves {
                loc: addr,
                first_move,
                moves: bwd_moves,
            });
            skip_first_case = true;
        }
    }

    ret
}

fn follow_fwd(gcs: &[GC], addr: Addr) -> Vec<Addr> {
    // println!("follow_fwd: gcs={:#?}, addr={:#?}", gcs, addr);

    let mut ret = vec![];

    for gc in gcs {
        match gc.moves_fwd.get(&addr) {
            None => {
                break;
            }
            Some(next_addr) => {
                ret.push(next_addr.addr);
            }
        }
    }

    ret
}

fn follow_bwd(gcs: &[GC], addr: Addr) -> Vec<Addr> {
    // println!("follow_bwd: gcs={:#?}, addr={:#?}", gcs, addr);

    let mut ret = vec![];

    for gc in gcs.iter().rev() {
        match gc.moves_bwd.get(&addr) {
            None => {
                break;
            }
            Some(prev_addr) => {
                ret.push(prev_addr.addr);
            }
        }
    }

    ret
}

//
// Tests
//

#[test]
fn parse_test() {
    let input = "\
        >>> GC 1\n\
        >>> 0x123 -> 0x124 size: 1\n\
        >>> 0x122 -> 0x123 size: 2\n\
        >>> GC 2\n\
    ";

    let gcs = parse(input.as_bytes());
    assert_eq!(gcs.len(), 2);
    assert_eq!(
        gcs[0].moves_fwd.get(&Addr(0x123)),
        Some(&AddrSize {
            addr: Addr(0x124),
            size: 1
        })
    );
    assert_eq!(
        gcs[0].moves_fwd.get(&Addr(0x122)),
        Some(&AddrSize {
            addr: Addr(0x123),
            size: 2
        })
    );
    assert_eq!(
        gcs[0].moves_bwd.get(&Addr(0x124)),
        Some(&AddrSize {
            addr: Addr(0x123),
            size: 1
        })
    );
    assert_eq!(
        gcs[0].moves_bwd.get(&Addr(0x123)),
        Some(&AddrSize {
            addr: Addr(0x122),
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
            loc: Addr(0x123),
            first_move: 0,
            moves: vec![Addr(0x123), Addr(0x124), Addr(0x125)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x100),
        vec![Moves {
            loc: Addr(0x100),
            first_move: 1,
            moves: vec![Addr(0x100), Addr(0x101)],
        }]
    );

    //
    // Test bwd search
    //

    assert_eq!(
        find_moves(&gcs, 0x101),
        vec![Moves {
            loc: Addr(0x101),
            first_move: 1,
            moves: vec![Addr(0x100), Addr(0x101)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x125),
        vec![Moves {
            loc: Addr(0x125),
            first_move: 0,
            moves: vec![Addr(0x123), Addr(0x124), Addr(0x125)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x124),
        vec![Moves {
            loc: Addr(0x124),
            first_move: 0,
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
        >>> 0x123 -> 0x124 size: 2\n\
    ";

    let gcs = parse(input.as_bytes());

    assert_eq!(
        find_moves(&gcs, 0x124),
        vec![
            Moves {
                loc: Addr(0x124),
                first_move: 0,
                moves: vec![Addr(0x124), Addr(0x125)],
            },
            Moves {
                loc: Addr(0x124),
                first_move: 0,
                moves: vec![Addr(0x123), Addr(0x124)],
            }
        ]
    );

    assert_eq!(
        find_moves(&gcs, 0x125),
        vec![Moves {
            loc: Addr(0x125),
            first_move: 0,
            moves: vec![Addr(0x124), Addr(0x125)],
        }]
    );

    assert_eq!(
        find_moves(&gcs, 0x123),
        vec![Moves {
            loc: Addr(0x123),
            first_move: 0,
            moves: vec![Addr(0x123), Addr(0x124)],
        }]
    );
}
