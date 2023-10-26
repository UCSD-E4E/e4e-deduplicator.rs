use clap::Parser;
use directories::ProjectDirs;
use std::path::{PathBuf, Path};
use walkdir::{IntoIter, WalkDir};
mod file_filter;
use file_filter::file_filter::FileFilter;
mod hash;
use hash::md5_digest;
use std::collections::HashMap;
use std::collections::hash_set::HashSet;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    job_name: String,

    #[arg(short, long)]
    working_directory: PathBuf,

    #[arg(short, long)]
    ignore_file: PathBuf,
}

fn main() {
    let args = Args::parse();
    let ignore_file_path = args.ignore_file;

    let base_dirs = ProjectDirs::from("edu.ucsd", "e4e", "deduplicator")
        .expect("Unable to create project dirs!");

    let data_dir = base_dirs.data_local_dir();

    let working_dir = args.working_directory;
    if !working_dir.exists() {
        println!(
            "Working directory {} does not exist!",
            working_dir.to_str().unwrap()
        );
        return;
    }

    let ignore_filter = match FileFilter::from_gitignore(ignore_file_path.as_path()) {
        Ok(ignore_filter) => ignore_filter,
        Err(_) => return,
    };

    let walker: IntoIter = WalkDir::new(working_dir).into_iter();

    let mut hashes: HashMap<String, HashSet<String>> = HashMap::new();

    for entry in walker {
        let (entry, digest) = match compute_digest(entry, &ignore_filter) {
            Some(value) => value,
            None => continue,
        };
        if !hashes.contains_key(&digest) {
            hashes.insert(digest.clone(), HashSet::new());
        }
        hashes.get_mut(&digest).unwrap().insert(entry.path().display().to_string());
    }
}

fn compute_digest(entry: Result<walkdir::DirEntry, walkdir::Error>, ignore_filter: &FileFilter) -> Option<(walkdir::DirEntry, String)> {
    let entry = match entry {
        Err(_) => return None,
        Ok(entry) => entry,
    };
    let ignore_match = match ignore_filter.matches(entry.path()) {
        Err(_) => return None,
        Ok(result) => result,
    };
    if ignore_match {
        return None;
    }
    if entry.path().is_dir() {
        return None;
    }
    let digest = md5_digest(entry.path()).unwrap();
    Some((entry, digest))
}
