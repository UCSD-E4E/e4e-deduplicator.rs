use clap::{ArgAction, Parser};
use directories::ProjectDirs;
use std::boxed::Box;
use std::collections::hash_set::HashSet;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::BufWriter;
use std::{
    fs::{create_dir_all, File},
    io::Stdout,
    io::{prelude::*, stdout},
    path::{Path, PathBuf},
};
use walkdir::{IntoIter, WalkDir};
mod file_filter;
use file_filter::file_filter::FileFilter;
mod hash;
use hash::md5_digest;
#[allow(unused_imports)]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use indicatif::ParallelProgressIterator;
use rayon::iter::{ParallelIterator, IntoParallelIterator};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    job_name: String,

    #[arg(short, long)]
    working_directory: PathBuf,

    #[arg(short, long)]
    ignore_file: PathBuf,

    #[arg(action=ArgAction::SetTrue, short, long)]
    clear_cache: bool,

    #[arg(short, long, required = false, default_value = "stdout")]
    analysis_dest: String,
}

#[derive(Serialize, Deserialize)]
struct DataSignature {
    hash: String,
    files: Vec<String>,
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

    let walker: IntoIter = WalkDir::new(working_dir.clone()).into_iter();
    let num_files: usize = walker.count().try_into().unwrap();
    println!("{} files to process", num_files);
    let walker: Vec<Result<walkdir::DirEntry, walkdir::Error>> = WalkDir::new(working_dir).into_iter().collect();

    let mut hashes: HashMap<String, HashSet<String>> = HashMap::new();

    if !args.clear_cache {
        if job_path.exists() {
            load_job_data(&job_path, &mut hashes).expect("Failed to load job data");
        }
    }
    let parallel_iterator = walker.into_par_iter().progress_count(num_files as u64).map(|x| compute_digest(x, &ignore_filter));
    let digest_results: Vec<Option<(walkdir::DirEntry, String)>> = parallel_iterator.collect();

    for result in digest_results {
        let (entry, digest) = match result {
            Some(value) => value,
            None => continue,
        };
        if !hashes.contains_key(&digest) {
            hashes.insert(digest.clone(), HashSet::new());
        }
        let absolute_path = match entry.path().canonicalize() {
            Ok(path) => path,
            Err(_) => {
                dbg!(entry);
                continue
            }
        };
        hashes
            .get_mut(&digest)
            .unwrap()
            .insert(absolute_path.display().to_string());
    }

    let mut output_writer: BufWriter<Box<dyn Write>> =
        BufWriter::new(match args.analysis_dest.as_str() {
            "stdout" => Box::new(stdout()),
            path => Box::new(File::create(path).unwrap()),
        });

    for (hash, files) in &hashes {
        if files.len() > 1 {
            writeln!(
                &mut output_writer,
                "File signature {} discovered {} times:",
                hash,
                files.len()
            )
            .unwrap();
            for file in files {
                writeln!(&mut output_writer, "\t{}", file).unwrap();
            }
        }
    }

    dump_job_data(&job_path, &hashes).expect("Failed to update job data");
}

fn compute_digest(
    entry: Result<walkdir::DirEntry, walkdir::Error>,
    ignore_filter: &FileFilter,
) -> Option<(walkdir::DirEntry, String)> {
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

fn load_job_data(
    job_path: &Path,
    current_hashes: &mut HashMap<String, HashSet<String>>,
) -> Result<(), std::io::Error> {
    let job_data = match read_to_string(job_path) {
        Err(err) => return Err(err),
        Ok(data) => data,
    };
    let data: Vec<DataSignature> = serde_json::from_str(&job_data)?;
    for entry in data {
        current_hashes.insert(entry.hash.clone(), HashSet::from_iter(entry.files));
    }
    return Ok(());
}

fn dump_job_data(
    job_path: &Path,
    current_hashes: &HashMap<String, HashSet<String>>,
) -> Result<(), std::io::Error> {
    let mut json_data: Vec<DataSignature> = Vec::new();
    for (hash, files) in current_hashes {
        json_data.push(DataSignature {
            hash: hash.clone(),
            files: Vec::from_iter(files.clone()),
        });
    }
    let json_str = serde_json::to_string_pretty(&json_data).expect("Failed to serialize data");
    let job_path_parent = job_path
        .parent()
        .expect("Unable to find job path directory");
    if !job_path_parent.exists() {
        create_dir_all(job_path_parent).expect("Unable to create job file directory");
    }
    let mut handle = File::create(job_path).expect("Failed to open job file for writing");
    handle
        .write_all(json_str.as_bytes())
        .expect("Failed to write data to job file");
    return Ok(());
}
