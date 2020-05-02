#[macro_use]
extern crate lazy_static;

use regex::Regex;

mod z_decode;
mod z_encode;

pub use z_decode::z_decode;
pub use z_encode::z_encode;

#[derive(Debug, PartialEq, Eq)]
pub struct GhcSummary {
    pub allocs: u64,
    pub gcs: u64,
    pub avg_res: u64,
    pub max_res: u64,
    pub in_use: u64,
}

lazy_static! {
    static ref GHC_SUMMARY_RE: Regex = Regex::new(
        r"<<ghc: (?P<allocs>\d+) bytes, (?P<gcs>\d+) GCs, (?P<avg_res>\d+)/(?P<max_res>\d+) .* (?P<in_use>\d+)M in use").unwrap();
}

pub fn parse_ghc_summary(s: &str) -> GhcSummary {
    let captures = GHC_SUMMARY_RE.captures(s);
    // println!("{:#?}", captures);

    let captures = captures.unwrap();

    GhcSummary {
        allocs: captures["allocs"].parse().unwrap(),
        gcs: captures["gcs"].parse().unwrap(),
        avg_res: captures["avg_res"].parse().unwrap(),
        max_res: captures["max_res"].parse().unwrap(),
        in_use: captures["in_use"].parse().unwrap(),
    }
}

#[test]
fn ghc_summary_parsing() {
    assert_eq!(
        parse_ghc_summary(
            "<<ghc: 3227088 bytes, 4 GCs, 200584/234944 avg/max bytes residency (2 samples), \
            2M in use, 0.000 INIT (0.000 elapsed), 0.001 MUT (0.002 elapsed), \
            0.004 GC (0.007 elapsed) :ghc>>"
        ),
        GhcSummary {
            allocs: 3227088,
            gcs: 4,
            avg_res: 200584,
            max_res: 234944,
            in_use: 2
        }
    );
}
