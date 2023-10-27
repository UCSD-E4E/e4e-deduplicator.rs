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
#[allow(unused_imports)]
use rayon::prelude::*;
use serde::{Serialize, Deserialize};
use std::fs::read_to_string;

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

#[derive(Serialize, Deserialize)]
struct DataSignature {
    hash: String,
    files: Vec<String>
}

fn main() {
    let args = Args::parse();
    let ignore_file_path = args.ignore_file;

    let base_dirs = ProjectDirs::from("edu.ucsd", "e4e", "deduplicator")
        .expect("Unable to create project dirs!");

    let data_dir = base_dirs.data_local_dir();
    let job_path = data_dir.join(args.job_name).with_extension("json");

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

    if job_path.exists() {
        load_job_data(&job_path, &mut hashes).expect("Failed to load job data");
    }


    let parallel_iterator = walker.map(|x| compute_digest(x, &ignore_filter));
    let digest_results: Vec<Option<(walkdir::DirEntry, String)>> = parallel_iterator.collect();

    for result in digest_results {
        let (entry, digest) = match result {
            Some(value) => value,
            None => continue,
        };
        if !hashes.contains_key(&digest) {
            hashes.insert(digest.clone(), HashSet::new());
        }
        hashes.get_mut(&digest).unwrap().insert(entry.path().canonicalize().unwrap().display().to_string());
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

fn load_job_data(job_path: &Path, current_hashes: &mut HashMap<String, HashSet<String>>) -> Result<(),()> {
    let job_data = read_to_string(job_path);
    return Ok(())
}