use clap::Parser;
use colored::*;
use git2::Repository;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "struct")]
#[command(about = "A smarter tree command with intelligent defaults", long_about = None)]
struct Args {
    /// Maximum depth to display (like tree -L)
    #[arg(value_name = "DEPTH")]
    depth: Option<usize>,

    /// Show git-tracked files only
    #[arg(short = 'g', long = "git")]
    git_mode: bool,

    /// Custom ignore patterns (comma-separated, e.g., "*.log,temp*")
    #[arg(short = 'i', long = "ignore")]
    ignore_patterns: Option<String>,

    /// Skip folders larger than SIZE MB
    #[arg(short = 's', long = "skip-large")]
    max_size_mb: Option<u64>,

    /// Starting directory
    #[arg(default_value = ".")]
    path: PathBuf,
}

struct StructConfig {
    depth: usize,
    custom_ignores: Vec<Regex>,
    max_size_bytes: Option<u64>,
    git_files: Option<HashSet<PathBuf>>,
}

fn main() {
    let args = Args::parse();

    let depth = args.depth.unwrap_or(3);
    let max_size_bytes = args.max_size_mb.map(|mb| mb * 1024 * 1024);

    // Parse custom ignore patterns
    let mut custom_ignores = Vec::new();
    if let Some(patterns) = args.ignore_patterns {
        for pattern in patterns.split(',') {
            let pattern = pattern.trim().replace("*", ".*");
            if let Ok(re) = Regex::new(&format!("^{}$", pattern)) {
                custom_ignores.push(re);
            }
        }
    }

    // Get git-tracked files if in git mode
    let git_files = if args.git_mode {
        get_git_tracked_files(&args.path)
    } else {
        None
    };

    let config = StructConfig {
        depth,
        custom_ignores,
        max_size_bytes,
        git_files,
    };

    println!("{}", args.path.display().to_string().cyan());
    display_tree(&args.path, &config, 0, "", true);
}

fn get_git_tracked_files(path: &Path) -> Option<HashSet<PathBuf>> {
    if let Ok(repo) = Repository::discover(path) {
        let mut tracked = HashSet::new();
        
        if let Ok(workdir) = repo.workdir().ok_or("No workdir") {
            if let Ok(index) = repo.index() {
                for entry in index.iter() {
                    if let Some(path_str) = std::str::from_utf8(&entry.path).ok() {
                        let full_path = workdir.join(path_str);
                        tracked.insert(full_path);
                    }
                }
            }
        }
        
        Some(tracked)
    } else {
        None
    }
}

fn should_ignore_dir(name: &str) -> bool {
    matches!(
        name,
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache" |
        ".tox" | "dist" | "build" | ".coverage" |
        "venv" | ".venv" | "env" | ".env" | "virtualenv" |
        "node_modules" | ".npm" | ".yarn" |
        ".git" | ".svn" | ".hg" |
        ".vscode" | ".idea" |
        "target" | "bin" | "obj" | ".next" | ".nuxt" |
        ".DS_Store"
    ) || name.ends_with(".egg-info")
}

fn should_ignore_file(name: &str) -> bool {
    matches!(
        name.split('.').last().unwrap_or(""),
        "pyc" | "pyo" | "pyd" | "swp" | "swo"
    ) || name == "package-lock.json" || name == ".DS_Store"
}

fn matches_custom_pattern(name: &str, patterns: &[Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(name))
}

fn get_dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

fn display_tree(
    path: &Path,
    config: &StructConfig,
    current_depth: usize,
    prefix: &str,
    _is_last: bool,
) {
    if current_depth >= config.depth {
        return;
    }

    let mut entries: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    // Sort: directories first, then alphabetically
    entries.sort_by_key(|e| {
        let path = e.path();
        let is_dir = path.is_dir();
        let name = e.file_name().to_string_lossy().to_lowercase();
        (!is_dir, name)
    });

    let total = entries.len();

    for (idx, entry) in entries.iter().enumerate() {
        let is_last_entry = idx == total - 1;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();

        // Check if we should skip this entry
        if is_dir && should_ignore_dir(&name) {
            // Count files in ignored directory
            let ignored_count = WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count();

            let connector = if is_last_entry { "└── " } else { "├── " };
            let dir_name = format!("{}/", name).blue().bold();
            let count_msg = format!(" ({} files ignored)", ignored_count).bright_black();
            println!("{}{}{}{}", prefix, connector, dir_name, count_msg);
            continue;
        }

        // Check custom ignore patterns
        if matches_custom_pattern(&name, &config.custom_ignores) {
            continue;
        }

        // Check git mode
        if let Some(ref git_files) = config.git_files {
            if !is_dir && !git_files.contains(&path) {
                continue;
            }
        }

        // Check file ignores
        if !is_dir && should_ignore_file(&name) {
            continue;
        }

        // Check size limit for directories
        if is_dir {
            if let Some(max_size) = config.max_size_bytes {
                let size = get_dir_size(&path);
                if size > max_size {
                    let connector = if is_last_entry { "└── " } else { "├── " };
                    let dir_name = format!("{}/", name).blue().bold();
                    let size_mb = size / (1024 * 1024);
                    let size_msg = format!(" ({}MB, skipped)", size_mb).bright_black();
                    println!("{}{}{}{}", prefix, connector, dir_name, size_msg);
                    continue;
                }
            }
        }

        // Display the entry
        let connector = if is_last_entry { "└── " } else { "├── " };
        let display_name = if is_dir {
            format!("{}/", name).blue().bold()
        } else if is_executable(&path) {
            name.green().bold()
        } else {
            name.normal()
        };

        println!("{}{}{}", prefix, connector, display_name);

        // Recurse into directories
        if is_dir {
            let new_prefix = if is_last_entry {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            display_tree(&path, config, current_depth + 1, &new_prefix, is_last_entry);
        }
    }
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            return permissions.mode() & 0o111 != 0;
        }
    }
    
    #[cfg(not(unix))]
    {
        // On Windows, check common executable extensions
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            return matches!(ext.as_str(), "exe" | "bat" | "cmd" | "sh" | "py" | "ps1");
        }
    }
    
    false
}