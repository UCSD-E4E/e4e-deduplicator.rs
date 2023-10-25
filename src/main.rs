use clap::Parser;
use directories::ProjectDirs;
use std::path::PathBuf;
use walkdir::{IntoIter, WalkDir};
mod file_filter;
use file_filter::file_filter::FileFilter;

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

    let base_dirs = ProjectDirs::from("edu.ucsd", "e4e", "deduplicator").expect("Unable to create project dirs!");

    let data_dir = base_dirs.data_local_dir();

    let working_dir = args.working_directory;
    if !working_dir.exists()
    {
        println!("Working directory {} does not exist!", working_dir.to_str().unwrap());
        return;
    }

    let ignore_filter = match FileFilter::from_gitignore(ignore_file_path.as_path()) {
        Ok(ignore_filter) => ignore_filter,
        Err(_) => return
    };

    let walker: IntoIter = WalkDir::new(working_dir).into_iter();

}
