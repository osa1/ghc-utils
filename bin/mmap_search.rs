use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::{App, Arg};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Addr(u64);

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("0x")?;
        fmt::LowerHex::fmt(&self.0, f)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct MapEvent {
    kind: MapKind,
    address: Addr,
    size: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum MapKind {
    Map,
    Unmap,
}

fn parse_mmap_line(line: &str) -> Option<MapEvent> {
    let line = line.trim();

    if !line.starts_with("mmap(") {
        return None;
    }

    let mut arg_iter = line.split_terminator(',');
    arg_iter.next(); // skip first arg
    let size = match arg_iter.next() {
        None => {
            return None;
        }
        Some(size) => match str::parse::<u64>(size.trim()) {
            Err(_) => {
                return None;
            }
            Ok(size) => size,
        },
    };

    let mut rsplit = line.rsplit(" = ");
    let address = match rsplit.next() {
        None => {
            return None;
        }
        Some(addr) => {
            // Drop 0x prefix before parsing
            match u64::from_str_radix(&addr[2..], 16) {
                Err(err) => {
                    eprintln!("Can't parse address {}: {}", addr, err);
                    return None;
                }
                Ok(addr) => Addr(addr),
            }
        }
    };

    Some(MapEvent {
        kind: MapKind::Map,
        address,
        size,
    })
}

fn parse_munmap_line(line: &str) -> Option<MapEvent> {
    let line = line.trim();

    if !line.starts_with("munmap(") {
        return None;
    }

    let line = &line["munmap(".len()..];

    let mut arg_iter = line.split_terminator(", ");
    let address = match arg_iter.next() {
        None => {
            return None;
        }
        Some(addr) => match u64::from_str_radix(&addr[2..], 16) {
            Err(err) => {
                eprintln!("Can't parse address {}: {}", addr, err);
                return None;
            }
            Ok(addr) => Addr(addr),
        },
    };

    let size = match arg_iter.next() {
        None => {
            return None;
        }
        Some(rest) => match rest.split(')').next() {
            None => {
                return None;
            }
            Some(size) => match str::parse::<u64>(size) {
                Err(_) => {
                    return None;
                }
                Ok(size) => size,
            },
        },
    };

    Some(MapEvent {
        kind: MapKind::Unmap,
        address,
        size,
    })
}

fn main() {
    let args = App::new("mmap-search")
        .about("Given a `strace -e trace=%memory` output and a address, finds which mmap/unmap calls map and unmap the address.")
        .arg(Arg::with_name("mmap-file").takes_value(true).required(true))
        .arg(Arg::with_name("address").takes_value(true).required(true))
        .get_matches();

    let mmap_file = args.value_of("mmap-file").unwrap();
    let addr = args.value_of("address").unwrap();

    let address = match u64::from_str_radix(&addr[2..], 16) {
        Err(err) => {
            eprintln!("Can't parse address: {}", err);
            ::std::process::exit(1);
        }
        Ok(address) => address,
    };

    let f = File::open(mmap_file).unwrap();
    let f = BufReader::new(f);
    for (line_idx, line) in f.lines().enumerate() {
        let line = line.unwrap();
        if let Some(map_event) = parse_mmap_line(&line).or_else(|| parse_munmap_line(&line)) {
            let start = map_event.address.0;
            let end = map_event.address.0 + map_event.size;
            if address >= start && address < end {
                println!("{}: {:?}", line_idx + 1, map_event);
            }
        }
    }
}

#[test]
fn mmap_parse_1() {
    // strace format
    assert_eq!(
        parse_mmap_line("mmap(NULL, 278570, PROT_READ, MAP_PRIVATE, 3, 0) = 0x7fe287e65000\n"),
        Some(MapEvent {
            kind: MapKind::Map,
            address: Addr(0x7fe287e65000),
            size: 278570
        })
    );
}

#[test]
fn mmap_parse_2() {
    // Custom format
    assert_eq!(
        parse_mmap_line("mmap((nil), 1600,) = 0x7fc69d0bd000\n"),
        Some(MapEvent {
            kind: MapKind::Map,
            address: Addr(0x7fc69d0bd000),
            size: 1600,
        })
    );
}

#[test]
fn munmap_parse() {
    assert_eq!(
        parse_munmap_line("munmap(0x40ac8000, 4096)        =         0\n"),
        Some(MapEvent {
            kind: MapKind::Unmap,
            address: Addr(0x40ac8000),
            size: 4096
        })
    );

    assert_eq!(
        parse_munmap_line("munmap(0x7f6aae93f000, 1600)\n"),
        Some(MapEvent {
            kind: MapKind::Unmap,
            address: Addr(0x7f6aae93f000),
            size: 1600
        })
    );
}
