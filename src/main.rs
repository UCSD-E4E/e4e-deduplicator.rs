use clap::Parser;
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    job_name: String,

    #[arg(short, long)]
    working_directory: PathBuf,
}

fn main() {
    let args = Args::parse();

    let base_dirs = ProjectDirs::from("edu.ucsd", "e4e", "deduplicator").unwrap();

    let data_dir = base_dirs.data_local_dir();

    println!("{:?}", args.job_name);
    println!("{:?}", args.working_directory);
    println!("{:?}", data_dir)
}
