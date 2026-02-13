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

fn search_files(pattern: &str, start_path: &Path, max_depth: usize, flat: bool) {
    // Convert glob pattern to regex
    let regex_pattern = pattern.replace("*", ".*").replace("?", ".");
    let re = match Regex::new(&format!("^{}$", regex_pattern)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("invalid pattern: {}", e);
            return;
        }
    };

    let mut found_count = 0;
    let mut matching_paths: HashSet<PathBuf> = HashSet::new();
    let mut flat_results: Vec<(PathBuf, u64)> = Vec::new();

    // Search through all files
    for entry in WalkDir::new(start_path)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| {
            // Skip common ignore directories to make search faster
            if let Some(name) = e.file_name().to_str() {
                !should_ignore_dir(name)
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(filename) = entry.file_name().to_str() {
                if re.is_match(filename) {
                    let file_path = entry.path().to_path_buf();
                    
                    if flat {
                        // For flat output, just store path and size
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        flat_results.push((file_path, size));
                    } else {
                        // For tree output, store path and all parent directories
                        matching_paths.insert(file_path.clone());
                        
                        // Add all parent directories
                        let mut current = file_path.parent();
                        while let Some(parent) = current {
                            if parent == start_path {
                                break;
                            }
                            matching_paths.insert(parent.to_path_buf());
                            current = parent.parent();
                        }
                    }
                    
                    found_count += 1;
                }
            }
        }
    }

    if found_count == 0 {
        println!("{}", format!("no files matching '{}' found", pattern).yellow());
        return;
    }

    println!("{} {}", format!("found {} file(s) matching", found_count).green(), pattern.cyan());
    println!();
    
    if flat {
        // Flat output: just list full paths
        flat_results.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, size) in flat_results {
            let size_str = format!(" ({})", format_size(size)).bright_black();
            println!("{}{}", path.display().to_string().cyan(), size_str);
        }
    } else {
        // Tree output
        display_search_tree(start_path, &matching_paths, 0, "", true);
    }
}

fn display_search_tree(
    path: &Path,
    matching_paths: &HashSet<PathBuf>,
    current_depth: usize,
    prefix: &str,
    _is_last: bool,
) {
    let mut entries: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let entry_path = e.path();
                // Only show entries that are in our matching set or are parents of matches
                matching_paths.contains(&entry_path) || 
                matching_paths.iter().any(|p| p.starts_with(&entry_path))
            })
            .collect(),
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
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry_path.is_dir();

        let connector = if is_last_entry { "└── " } else { "├── " };
        
        if is_dir {
            let dir_name = format!("{}/", name).blue().bold();
            println!("{}{}{}", prefix, connector, dir_name);
            
            let new_prefix = if is_last_entry {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            display_search_tree(&entry_path, matching_paths, current_depth + 1, &new_prefix, is_last_entry);
        } else {
            // This is a matching file
            let file_name = if is_executable(&entry_path) {
                name.green().bold()
            } else {
                name.cyan().bold()
            };
            
            if let Ok(metadata) = fs::metadata(&entry_path) {
                let size_str = format!(" ({})", format_size(metadata.len())).bright_black();
                println!("{}{}{}{}", prefix, connector, file_name, size_str);
            } else {
                println!("{}{}{}", prefix, connector, file_name);
            }
        }
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

    /// Disable ignores: 'all', 'defaults', 'config', or specific pattern
    #[arg(short = 'n', long = "no-ignore")]
    no_ignore: Option<String>,

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
    /// Search for files matching a pattern
    Search {
        /// Pattern to search for (e.g., "*.env", "config", "test*")
        pattern: String,
        /// Maximum depth to search (0 for infinite)
        #[arg(short = 'd', long = "depth", default_value = "0")]
        depth: usize,
        /// Flat output (show full paths instead of tree)
        #[arg(short = 'f', long = "flat")]
        flat: bool,
        /// Starting directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

struct StructConfig {
    depth: usize,
    custom_ignores: Vec<Regex>,
    max_size_bytes: Option<u64>,
    git_files: Option<HashSet<PathBuf>>,
    show_size: bool,
    skip_defaults: bool,
    _skip_config: bool,
    skip_specific: Option<String>,
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
            Commands::Search { pattern, depth, flat, path } => {
                let max_depth = if depth == 0 { usize::MAX } else { depth };
                search_files(&pattern, &path, max_depth, flat);
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

    // Parse no-ignore option
    let (skip_defaults, skip_config, skip_specific) = match args.no_ignore {
        Some(ref mode) => match mode.as_str() {
            "all" => (true, true, None),
            "defaults" => (true, false, None),
            "config" => (false, true, None),
            pattern => (false, false, Some(pattern.to_string())),
        },
        None => (false, false, None),
    };

    // Load config patterns
    let config_patterns = if skip_config {
        Vec::new()
    } else {
        load_config_patterns()
    };

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
        skip_defaults,
        _skip_config: skip_config,
        skip_specific,
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
        if is_dir {
            let should_skip = if config.skip_defaults {
                false
            } else if let Some(ref specific) = config.skip_specific {
                // Only ignore if it matches the specific pattern
                &name == specific
            } else {
                should_ignore_dir(&name)
            };

            if should_skip {
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
        }

        // Check custom ignore patterns (unless we have a specific skip pattern)
        if config.skip_specific.is_none() && matches_custom_pattern(&name, &config.custom_ignores) {
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