use clap::Parser;
use colored::*;
use git2::Repository;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("struct").join("ignores.txt")
}

fn load_config_patterns() -> Vec<String> {
    let config_path = get_config_path();
    if let Ok(content) = fs::read_to_string(&config_path) {
        content.lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .collect()
    } else {
        Vec::new()
    }
}

fn save_config_patterns(patterns: &[String]) -> std::io::Result<()> {
    let config_path = get_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config_path, patterns.join("\n"))
}

fn add_config_pattern(pattern: String) {
    let mut patterns = load_config_patterns();
    if patterns.contains(&pattern) {
        println!("{} already in config", pattern.yellow());
        return;
    }
    patterns.push(pattern.clone());
    if let Err(e) = save_config_patterns(&patterns) {
        eprintln!("failed to save config: {}", e);
        return;
    }
    println!("{} added to config", pattern.green());
    println!("config file: {}", get_config_path().display().to_string().bright_black());
}

fn remove_config_pattern(pattern: String) {
    let mut patterns = load_config_patterns();
    let before_len = patterns.len();
    patterns.retain(|p| p != &pattern);
    
    if patterns.len() == before_len {
        println!("{} not found in config", pattern.yellow());
        return;
    }
    
    if let Err(e) = save_config_patterns(&patterns) {
        eprintln!("failed to save config: {}", e);
        return;
    }
    println!("{} removed from config", pattern.red());
}

fn list_config_patterns() {
    let patterns = load_config_patterns();
    if patterns.is_empty() {
        println!("no custom patterns configured");
        println!("add some with: struct add \"pattern\"");
        return;
    }
    
    println!("{}", "custom ignore patterns:".bright_black());
    for pattern in patterns {
        println!("  {}", pattern.cyan());
    }
    println!("\nconfig file: {}", get_config_path().display().to_string().bright_black());
}

fn clear_config_patterns() {
    let config_path = get_config_path();
    if config_path.exists() {
        if let Err(e) = fs::remove_file(&config_path) {
            eprintln!("failed to clear config: {}", e);
            return;
        }
        println!("{}", "cleared all custom patterns".green());
    } else {
        println!("no config file to clear");
    }
}

#[derive(Parser, Debug)]
#[command(name = "struct")]
#[command(about = "A smarter tree command with intelligent defaults", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

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

    /// Show file sizes
    #[arg(short = 'z', long = "size")]
    show_size: bool,

    /// Starting directory
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Add a pattern to the config file
    Add {
        /// Pattern to add (e.g., "cache", "*.log")
        pattern: String,
    },
    /// Remove a pattern from the config file
    Remove {
        /// Pattern to remove
        pattern: String,
    },
    /// List all custom ignore patterns
    List,
    /// Clear all custom ignore patterns
    Clear,
}

struct StructConfig {
    depth: usize,
    custom_ignores: Vec<Regex>,
    max_size_bytes: Option<u64>,
    git_files: Option<HashSet<PathBuf>>,
    show_size: bool,
}

fn main() {
    let args = Args::parse();

    // Handle subcommands
    if let Some(command) = args.command {
        match command {
            Commands::Add { pattern } => {
                add_config_pattern(pattern);
                return;
            }
            Commands::Remove { pattern } => {
                remove_config_pattern(pattern);
                return;
            }
            Commands::List => {
                list_config_patterns();
                return;
            }
            Commands::Clear => {
                clear_config_patterns();
                return;
            }
        }
    }

    // Depth 0 means infinite, otherwise use provided depth or default to 3
    let depth = match args.depth {
        Some(0) => usize::MAX,  // Infinite depth
        Some(d) => d,
        None => 3,              // Default depth
    };
    
    let max_size_bytes = args.max_size_mb.map(|mb| mb * 1024 * 1024);

    // Load config patterns
    let config_patterns = load_config_patterns();

    // Parse custom ignore patterns (from -i flag)
    let mut custom_ignores = Vec::new();
    
    // Add config file patterns
    for pattern in config_patterns {
        let pattern = pattern.replace("*", ".*");
        if let Ok(re) = Regex::new(&format!("^{}$", pattern)) {
            custom_ignores.push(re);
        }
    }
    
    // Add command-line patterns
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
        show_size: args.show_size,
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
        ".vscode" | ".idea" | ".obsidian" |
        "target" | "bin" | "obj" | ".next" | ".nuxt" |
        ".DS_Store" |
        "chrome_profile" | "lofi_chrome_profile" |
        "GPUCache" | "ShaderCache" | "GrShaderCache" |
        "Cache" | "blob_storage"
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

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
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
            
            if config.show_size {
                let size = get_dir_size(&path);
                let size_str = format_size(size);
                let count_msg = format!(" ({}, {} files ignored)", size_str, ignored_count).bright_black();
                println!("{}{}{}{}", prefix, connector, dir_name, count_msg);
            } else {
                let count_msg = format!(" ({} files ignored)", ignored_count).bright_black();
                println!("{}{}{}{}", prefix, connector, dir_name, count_msg);
            }
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

        // Add size if requested
        if config.show_size {
            if is_dir {
                println!("{}{}{}", prefix, connector, display_name);
            } else {
                if let Ok(metadata) = fs::metadata(&path) {
                    let size_str = format!(" ({})", format_size(metadata.len())).bright_black();
                    println!("{}{}{}{}", prefix, connector, display_name, size_str);
                } else {
                    println!("{}{}{}", prefix, connector, display_name);
                }
            }
        } else {
            println!("{}{}{}", prefix, connector, display_name);
        }

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