use clap::{ArgAction, Parser, Subcommand};
use directories::ProjectDirs;
use std::boxed::Box;
use std::collections::hash_set::HashSet;
use std::collections::HashMap;
use std::fs::{read_to_string, remove_file};
use std::io::BufWriter;
use std::{
    fs::{create_dir_all, File},
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
use indicatif::{ParallelProgressIterator, ProgressStyle};
use rayon::iter::{ParallelIterator, IntoParallelIterator};

#[derive(Subcommand, Debug)]
enum Commands {
    /// Adds the file hashes of the working directory to the database
    Analyze {},
    /// Deletes any files that match existing entries in the database
    Delete {},
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

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

    #[arg(short, long, action=ArgAction::SetTrue)]
    fs_test: bool
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

    if args.command.is_none() {
        println!("No commands given!");
    }

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
    let style = ProgressStyle::with_template("[{elapsed_precise}] {wide_bar} {pos:>7}/{len:7} {eta} {msg}").unwrap();
    let parallel_iterator = walker.into_par_iter().progress_with_style(style).map(|x| compute_digest(x, &ignore_filter));
    let digest_results: Vec<Option<(walkdir::DirEntry, String)>> = parallel_iterator.collect();

    let mut output_writer: BufWriter<Box<dyn Write>> =
        BufWriter::new(match args.analysis_dest.as_str() {
            "stdout" => Box::new(stdout()),
            path => Box::new(File::create(path).unwrap()),
        });

    for result in &digest_results {
        let (entry, digest) = match result {
            Some(value) => value,
            None => continue,
        };
        let absolute_path = match entry.path().canonicalize() {
            Ok(path) => path,
            Err(_) => {
                dbg!(entry);
                continue
            }
        };
        match &args.command {
            Some(Commands::Analyze {  }) => {
                if !hashes.contains_key(&*digest) {
                    hashes.insert(digest.clone(), HashSet::new());
                }
                hashes
                    .get_mut(&*digest)
                    .unwrap()
                    .insert(absolute_path.display().to_string());
            }
            Some(Commands::Delete {  }) => {
                match hashes.get(&*digest) {
                    Some(hash_set) => {
                        if hash_set.contains(&absolute_path.display().to_string()) && hash_set.len() > 1{
                            // this hash is unique, continue
                            continue;
                        }
                    }
                    None => continue,
                }
                if !args.fs_test{
                    let result = remove_file(&absolute_path);
                    match result {
                        Ok(()) => {}
                        Err(..) => {
                            print!("Failed to remove {}", absolute_path.display().to_string());
                            continue;
                        }
                    }
                }
                writeln!(&mut output_writer, "Deleted hash {} at {}", &*digest, absolute_path.display().to_string()).unwrap();
            }
            None => {}
        }
    }

    match &args.command {
        Some(Commands::Analyze {  }) => {
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
        }
        Some(Commands::Delete {  }) | None => {}
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
