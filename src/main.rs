use clap::Parser;
use log::LevelFilter;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

static SNAPSHOT_SUFFIX: &str = "-SNAPSHOT";

static SUFFIXIES: [&str; 6] = [
    ".jar",
    ".jar.sha1",
    ".pom",
    ".pom.sha1",
    ".war",
    ".war.sha1",
];

static LOCAL_METADATA_FILE: &str = "maven-metadata-local.xml";

fn main() {
    let args = Args::parse();
    let level_filter = LevelFilter::from_str(&args.level).expect("Invalid log level");
    simple_logger::SimpleLogger::new()
        .with_level(level_filter)
        .without_timestamps()
        .init()
        .expect("Failed to initialize logger");
    let path = PathBuf::from(&args.path);
    if !path.exists() {
        log::error!("file or directory does not exist: {}", &args.path);
        process::exit(1);
    }
    if path.is_file() {
        log::error!("Maven Repo is not a file: {}", &args.path);
        process::exit(1);
    }

    log::info!("Cleaning up: {}", &args.path);
    cleanup(PathBuf::from(&args.path));
}

fn cleanup(repo_path: PathBuf) {
    let mut deleted_size: usize = 0;
    let mut queue = VecDeque::new();
    queue.push_back(repo_path);
    while let Some(path) = queue.pop_front() {
        if path.is_dir() {
            let folder_name = get_file_name(&path);
            if folder_name.is_none() {
                continue;
            }
            let folder_name = folder_name.unwrap();
            match std::fs::read_dir(path.as_path()) {
                Ok(folder) => {
                    for entry in folder {
                        if let Err(e) = entry {
                            log::error!("Failed to read directory entry: {:?}", e);
                            continue;
                        }

                        let entry = entry.unwrap();
                        let entry_path = entry.path();
                        if entry_path.is_file() {
                            // 跳过非快照文件
                            let entry_file_name = get_file_name(&entry_path).unwrap();
                            if folder_name.ends_with(SNAPSHOT_SUFFIX)
                                || entry_file_name.eq(LOCAL_METADATA_FILE)
                            {
                                queue.push_back(entry_path);
                            }
                        } else {
                            queue.push_back(entry_path);
                        }
                    }
                    log::debug!("Scanning: {}", path.display());
                }
                Err(e) => {
                    log::error!("Failed to read directory: {}", e);
                }
            }
        } else {
            let folder = path.parent();
            if folder.is_none() {
                continue;
            }
            let folder = folder.unwrap();
            let folder_name = get_file_name(folder);
            let file_name = get_file_name(&path);
            if folder_name.is_none() || file_name.is_none() {
                continue;
            }
            let folder_name = folder_name.unwrap();
            let file_name = file_name.unwrap();

            if LOCAL_METADATA_FILE.eq(&file_name) {
                log::info!("Deleting: {}", path.display());
                if let Err(e) = std::fs::remove_file(&path) {
                    log::error!("Failed to delete file '{}': {}", path.display(), e);
                    break;
                }
            } else {
                for suffix in SUFFIXIES {
                    if file_name.ends_with(suffix) && !file_name.contains(&folder_name) {
                        log::info!("Deleting: {}", path.display());
                        deleted_size += std::fs::metadata(&path)
                            .map(|metadata| metadata.len() as usize)
                            .unwrap_or(0);
                        if let Err(e) = std::fs::remove_file(&path) {
                            log::error!("Failed to delete file '{}': {}", path.display(), e);
                            break;
                        }
                    }
                }
            }
        }
    }

    let size_text = format_size(deleted_size);
    log::info!("Deleted size: {}", &size_text);
}

fn get_file_name(path: &Path) -> Option<String> {
    match path.file_name() {
        None => None,
        Some(folder_name) => folder_name
            .to_str()
            .map(|folder_name| folder_name.to_string()),
    }
}

fn format_size(size: usize) -> String {
    match size {
        s if s >= 1024 * 1024 * 1024 => format!("{:.2} GiB", s as f64 / (1024.0 * 1024.0 * 1024.0)),
        s if s >= 1024 * 1024 => format!("{:.2} MiB", s as f64 / (1024.0 * 1024.0)),
        s if s >= 1024 => format!("{:.2} KiB", s as f64 / 1024.0),
        s => format!("{} B", s),
    }
}

#[derive(Parser, Debug)]
#[command(author = "arsiac", version = "0.1.0", about = "Clean Maven Repository")]
pub struct Args {
    path: String,

    #[arg(long, default_value = "INFO")]
    level: String,
}
