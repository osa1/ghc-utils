//! Generates a Gitlab markdown table from a NoFib analyse output

use clap::{App, Arg};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

fn main() {
    let args = App::new("nofib-to-gitlab")
        .about("Generate human-readable Gitlab markdown tables for nofib-analyse outputs")
        .arg(
            Arg::with_name("nofib-analyse-out")
                .help("Path to nofib-analyse output")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let path = args.value_of("nofib-analyse-out").unwrap();
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

    let mut col_headers: Vec<String> = vec![];
    let mut rows: Vec<Vec<String>> = vec![];
    let mut summary: Vec<Vec<String>> = vec![];

    read_header(&mut reader, &mut col_headers);
    read_cols(&mut reader, &mut rows);
    read_summary(&mut reader, &mut summary);

    // println!("col_headers: {:?}", col_headers);
    // println!("rows: {:?}", rows);
    // println!("summary: {:?}", summary);

    let mut col_widths: Vec<usize> = vec![];
    for col in &col_headers {
        // Assuming ASCII
        col_widths.push(col.len() + 2);
    }

    for row in rows.iter().chain(summary.iter()) {
        for (col_idx, col) in row.iter().enumerate() {
            // Assuming ASCII
            col_widths[col_idx] = std::cmp::max(col_widths[col_idx], col.len() + 2);
        }
    }

    // println!("col_widths: {:?}", col_widths);

    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();
    print_cols(&col_headers, &col_widths, &mut stdout_lock);
    print_sep(&col_widths, &mut stdout_lock);
    for row in &rows {
        print_cols(row, &col_widths, &mut stdout_lock);
    }

    // TODO: This separator doesn't render as I expect
    print_sep(&col_widths, &mut stdout_lock);
    for row in &summary {
        print_cols(row, &col_widths, &mut stdout_lock);
    }
}

fn read_header(reader: &mut BufReader<File>, col_headers: &mut Vec<String>) {
    for line in reader.lines() {
        let line = line.unwrap();
        if is_line_sep(&line) {
            break;
        }
    }

    if let Some(line) = reader.lines().next() {
        let line = line.unwrap();
        for word in line.split_whitespace() {
            col_headers.push(word.trim().to_owned());
        }

        let _ = reader.lines().next();
    }
}

fn read_cols(reader: &mut BufReader<File>, cols: &mut Vec<Vec<String>>) {
    for line in reader.lines() {
        let line = line.unwrap();
        if is_line_sep(&line) {
            break;
        }
        cols.push(
            line.split_whitespace()
                .map(|s| s.trim().to_owned())
                .collect(),
        );
    }
}

fn read_summary(reader: &mut BufReader<File>, cols: &mut Vec<Vec<String>>) {
    // 3 rows: 'Min', 'Max', and 'Geometric Mean'
    let min_line = reader.lines().next().unwrap().unwrap();
    cols.push(
        min_line
            .split_whitespace()
            .map(|s| s.trim().to_owned())
            .collect(),
    );

    let max_line = reader.lines().next().unwrap().unwrap();
    cols.push(
        max_line
            .split_whitespace()
            .map(|s| s.trim().to_owned())
            .collect(),
    );

    let geo_mean_line = reader.lines().next().unwrap().unwrap();
    let mut v = vec!["Geometric Mean".to_owned()];
    for word in geo_mean_line.split_whitespace().skip(2) {
        v.push(word.trim().to_owned());
    }
    cols.push(v);
}

fn is_line_sep(str: &str) -> bool {
    !str.is_empty() && str.chars().all(|c| c == '-')
}

fn print_cols<W: Write>(row: &[String], widths: &[usize], w: &mut W) {
    for (width, col) in widths.iter().zip(row.iter()) {
        // Assuming ASCII
        let str_w = col.len();

        write!(w, "| ").unwrap();
        write!(w, "{}", col).unwrap();
        for _ in 0..width - str_w - 1 {
            write!(w, " ").unwrap();
        }
    }
    writeln!(w, "|").unwrap();
}

fn print_sep<W: Write>(widths: &[usize], w: &mut W) {
    write!(w, "|").unwrap();
    for width in widths {
        for _ in 0..*width {
            write!(w, "-").unwrap();
        }
        write!(w, "|").unwrap();
    }
    writeln!(w).unwrap();
}
