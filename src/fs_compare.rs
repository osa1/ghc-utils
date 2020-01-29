//! fs-compare <dir1> <dir2> [<file extension>]
//!
//! Compares sizes of files with the given extension (all files if extension is not given).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn file_sizes(root: &Path, dir: &Path, ext: &str, files: &mut HashMap<String, u64>) {
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
            if let Some(ext_found) = path.extension() {
                match ext_found.to_str() {
                    None => {
                        eprintln!("Can't convert Path extension to &str: {:?}", path);
                        continue;
                    }
                    Some(ext_found_str) => {
                        if ext_found_str == ext {
                            match dir_ent.metadata() {
                                Err(err) => {
                                    eprintln!(
                                        "Error when getting metadata of {:?}: {:?}",
                                        path, err
                                    );
                                    continue;
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
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn compare_files(f1: HashMap<String, u64>, mut f2: HashMap<String, u64>) {
    // bool: whether the file exists in both dirs
    let mut diffs: Vec<(String, i64, bool)> = Vec::with_capacity(std::cmp::max(f1.len(), f2.len()));

    for (k, v1) in f1.into_iter() {
        match f2.remove(&k) {
            None => {
                diffs.push((k, -(v1 as i64), false));
            }
            Some(v2) => {
                if v1 != v2 {
                    diffs.push((k, (v2 as i64) - (v1 as i64), true))
                }
            }
        }
    }

    for (k, v2) in f2.into_iter() {
        diffs.push((k, v2 as i64, false));
    }

    // Sort the vector based on diff size
    diffs.sort_by_key(|&(_, v, _)| std::cmp::Reverse(v));

    for (path, diff, exists_both) in diffs.into_iter() {
        let sign = if exists_both {
            '~'
        } else if diff > 0 {
            '+'
        } else {
            '-'
        };

        println!("[{}] {}: {:+}", sign, path, diff);
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let dir1 = &args[1];
    let dir2 = &args[2];
    let ext = &args[3];

    let mut files1 = HashMap::new();
    let dir1_path = Path::new(dir1);
    file_sizes(dir1_path, dir1_path, ext, &mut files1);

    let mut files2 = HashMap::new();
    let dir2_path = Path::new(dir2);
    file_sizes(dir2_path, dir2_path, ext, &mut files2);

    compare_files(files1, files2);
}
