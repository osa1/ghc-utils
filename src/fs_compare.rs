//! fs-compare <dir1> <dir2> [<file extension>]
//!
//! Compares sizes of files with the given extension (all files if extension is not given).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use clap::{App, Arg};

fn add_file(root: &Path, dir_ent: &fs::DirEntry, path: &Path, files: &mut HashMap<String, u64>) {
    match dir_ent.metadata() {
        Err(err) => {
            eprintln!("Error when getting metadata of {:?}: {:?}", path, err);
        }
        Ok(metadata) => {
            if metadata.is_file() {
                let size = metadata.len();
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    size,
                );
            }
        }
    }
}

fn file_sizes(root: &Path, dir: &Path, ext: Option<&str>, files: &mut HashMap<String, u64>) {
    for dir_ent in fs::read_dir(dir).unwrap() {
        let dir_ent = dir_ent.unwrap();
        let path = dir_ent.path();

        // Dir or file?
        let file_type = match dir_ent.file_type() {
            Err(err) => {
                eprintln!("Can't get file type of {:?}: {:?}", dir_ent, err);
                continue;
            }
            Ok(file_type) => file_type,
        };

        if file_type.is_dir() {
            file_sizes(root, &path, ext, files);
        } else {
            match ext {
                None => {
                    add_file(root, &dir_ent, &path, files);
                }
                Some(ext_wanted) => {
                    if let Some(ext_found) = path.extension() {
                        match ext_found.to_str() {
                            None => {
                                eprintln!("Can't convert Path extension to &str: {:?}", path);
                                continue;
                            }
                            Some(ext_found_str) => {
                                if ext_found_str == ext_wanted {
                                    add_file(root, &dir_ent, &path, files);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn compare_files(f1: HashMap<String, u64>, mut f2: HashMap<String, u64>, sort_p: bool) {
    // bool: whether the file exists in both dirs
    let mut diffs: Vec<(String, i64, Option<f64>, bool)> =
        Vec::with_capacity(std::cmp::max(f1.len(), f2.len()));

    for (k, v1) in f1.into_iter() {
        match f2.remove(&k) {
            None => {
                diffs.push((k, -(v1 as i64), None, false));
            }
            Some(v2) => {
                if v1 != v2 {
                    let diff = (v2 as i64) - (v1 as i64);
                    let p = ((diff as f64) / (v1 as f64)) * 100f64;
                    diffs.push((k, diff, Some(p), true))
                }
            }
        }
    }

    for (k, v2) in f2.into_iter() {
        diffs.push((k, v2 as i64, None, false));
    }

    // Sort the vector based on diff size or percentage
    if sort_p {
        diffs.sort_by(|&(_, _, p1, _), &(_, _, p2, _)| p2.partial_cmp(&p1).unwrap());
    } else {
        diffs.sort_by_key(|&(_, v, _, _)| std::cmp::Reverse(v));
    }

    for (path, diff, p, exists_both) in diffs.into_iter() {
        let sign = if exists_both {
            '~'
        } else if diff > 0 {
            '+'
        } else {
            '-'
        };

        match p {
            None => {
                println!("[{}] {}: {:+}", sign, path, diff);
            }
            Some(p) => {
                println!("[{}] {}: {:+} ({:.2}%)", sign, path, diff, p);
            }
        }
    }
}

fn main() {
    let args = App::new("fs-compare")
        .arg(Arg::with_name("dir_1").takes_value(true).required(true))
        .arg(Arg::with_name("dir_2").takes_value(true).required(true))
        .arg(Arg::with_name("ext").takes_value(true).required(false))
        .arg(
            Arg::with_name("sort_percentage")
                .help("Sort files by increase in percentage, rather than in bytes")
                .takes_value(false)
                .required(false)
                .short("p"),
        )
        .get_matches();

    let dir1 = args.value_of("dir_1").unwrap();
    let dir2 = args.value_of("dir_2").unwrap();
    let ext = args.value_of("ext");
    let sort_p = args.is_present("sort_percentage");

    let mut files1 = HashMap::new();
    let dir1_path = Path::new(dir1);
    file_sizes(dir1_path, dir1_path, ext, &mut files1);

    let mut files2 = HashMap::new();
    let dir2_path = Path::new(dir2);
    file_sizes(dir2_path, dir2_path, ext, &mut files2);

    compare_files(files1, files2, sort_p);
}
