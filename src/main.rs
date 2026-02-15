use clap::Parser;
use colored::*;
use git2::Repository;
use regex::Regex;
use std::path::PathBuf;

mod config;
mod display;
mod ignores;
mod search;
mod summary;
mod utils;

use crate::config::{add_config_pattern, clear_config_patterns, list_config_patterns, load_config_patterns, remove_config_pattern};
use display::{display_tree, get_git_tracked_files, get_git_untracked_files, get_git_staged_files, get_git_changed_files, GitMode, StructConfig};
use search::search_files;
use summary::display_summary;

#[derive(Parser, Debug)]
#[command(name = "struct")]
#[command(version)]
#[command(about = "A smarter tree command with intelligent defaults", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Maximum depth to display (0 = current dir only, default = infinite)
    #[arg(value_name = "DEPTH")]
    depth: Option<usize>,

    /// Starting directory
    #[arg(short = 'p', long = "path", default_value = ".")]
    path: PathBuf,

    /// Git mode: show only tracked files
    #[arg(short = 'g', long = "git")]
    git_tracked: bool,

    /// Git untracked: show only untracked files
    #[arg(long = "gu")]
    git_untracked: bool,

    /// Git staged: show only staged files
    #[arg(long = "gs")]
    git_staged: bool,

    /// Git changed: show modified files (not staged)
    #[arg(long = "gc")]
    git_changed: bool,

    /// Git history: show last commit per directory
    #[arg(long = "gh")]
    git_history: bool,

    /// Start from git root (use with -g, --gu, --gs, --gc, --gh)
    #[arg(long = "gr")]
    git_root: bool,

    #[arg(long = "gur")]
    git_untracked_root: bool,

    #[arg(long = "gsr")]
    git_staged_root: bool,

    #[arg(long = "gcr")]
    git_changed_root: bool,

    #[arg(long = "ghr")]
    git_history_root: bool,

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
                
                // Load config patterns for search
                let config_patterns = load_config_patterns();
                let mut custom_ignores = Vec::new();
                for pattern in config_patterns {
                    let pattern = pattern.replace("*", ".*");
                    if let Ok(re) = Regex::new(&format!("^{}$", pattern)) {
                        custom_ignores.push(re);
                    }
                }
                
                search_files(&pattern, &path, max_depth, flat, &custom_ignores);
                return;
            }
        }
    }

    // Depth: None = infinite, 0 = current dir only, otherwise use provided
    let depth = match args.depth {
        None => usize::MAX,         // No depth arg = infinite
        Some(0) => 1,                // 0 = current dir only  
        Some(d) => d,                // Use provided depth
    };
    
    let max_size_bytes = args.max_size_mb.map(|mb| mb * 1024 * 1024);

    // Determine git mode
    let git_mode = if args.git_changed || args.git_changed_root {
        Some(GitMode::Changed)
    } else if args.git_staged || args.git_staged_root {
        Some(GitMode::Staged)
    } else if args.git_untracked || args.git_untracked_root {
        Some(GitMode::Untracked)
    } else if args.git_tracked || args.git_root {
        Some(GitMode::Tracked)
    } else if args.git_history || args.git_history_root {
        Some(GitMode::History)
    } else {
        None
    };

    // Determine if we should start from git root
    let use_git_root = args.git_root || args.git_untracked_root 
        || args.git_staged_root || args.git_changed_root || args.git_history_root;

    // Check if in git repo when git mode is used
    if git_mode.is_some() {
        if Repository::discover(&args.path).is_err() {
            eprintln!("Not in a git repository");
            return;
        }
    }

    // Get the actual starting path
    let start_path = if use_git_root {
        // Find git root
        if let Ok(repo) = Repository::discover(&args.path) {
            if let Some(workdir) = repo.workdir() {
                workdir.to_path_buf()
            } else {
                args.path.clone()
            }
        } else {
            eprintln!("Not in a git repository");
            return;
        }
    } else {
        args.path.clone()
    };

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

    // Get git files if in git mode
    let git_files = if let Some(ref mode) = git_mode {
        match mode {
            GitMode::Tracked => get_git_tracked_files(&start_path),
            GitMode::Untracked => get_git_untracked_files(&start_path),
            GitMode::Staged => get_git_staged_files(&start_path),
            GitMode::Changed => get_git_changed_files(&start_path),
            GitMode::History => None, // History is handled differently
        }
    } else {
        None
    };

    let config = StructConfig {
        depth,
        custom_ignores,
        max_size_bytes,
        git_files,
        git_mode,
        show_size: args.show_size,
        skip_defaults,
        skip_specific,
    };

    // Special mode: depth 1 (struct 0) shows detailed summary
    if args.depth == Some(0) {
        display_summary(&start_path);
        return;
    }

    println!("{}", start_path.display().to_string().cyan());
    display_tree(&start_path, &config, 0, "", true);
}