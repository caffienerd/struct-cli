use colored::*;
use git2::Repository;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::ignores::{should_ignore_dir, should_ignore_file, matches_custom_pattern};
use crate::utils::{format_size, get_dir_size, is_executable};

pub struct StructConfig {
    pub depth: usize,
    pub custom_ignores: Vec<Regex>,
    pub max_size_bytes: Option<u64>,
    pub git_files: Option<HashSet<PathBuf>>,
    pub show_size: bool,
    pub skip_defaults: bool,
    pub skip_specific: Option<String>,
}

/// Get git-tracked files
pub fn get_git_tracked_files(path: &Path) -> Option<HashSet<PathBuf>> {
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

/// Display directory tree
pub fn display_tree(
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
        // Check if it's a symlink pointing to a directory
        let is_dir = if path.is_symlink() {
            // Don't follow symlinks to avoid infinite loops
            false
        } else {
            path.is_dir()
        };
        let name = e.file_name().to_string_lossy().to_lowercase();
        (!is_dir, name)
    });

    let total = entries.len();

    for (idx, entry) in entries.iter().enumerate() {
        let is_last_entry = idx == total - 1;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        
        // Check if it's a symlink first - NEVER recurse into symlinks
        let is_symlink = path.is_symlink();
        let is_dir = if is_symlink {
            false  // Treat symlinks as files to prevent recursion
        } else {
            path.is_dir()
        };

        // Check if we should skip this entry
        if is_dir {
            let should_skip = if config.skip_defaults {
                // -n defaults: don't ignore any defaults
                false
            } else if let Some(ref specific) = config.skip_specific {
                // -n PATTERN: only ignore if it DOESN'T match the specific pattern
                &name != specific && should_ignore_dir(&name)
            } else {
                // Normal mode: ignore defaults
                should_ignore_dir(&name)
            };

            if should_skip {
                // Count files in ignored directory
                let ignored_count = WalkDir::new(&path)
                    .follow_links(false)
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
        let display_name = if is_symlink {
            // Show symlink with arrow
            if let Ok(target) = fs::read_link(&path) {
                format!("{} -> {}", name, target.display()).cyan()
            } else {
                name.cyan()
            }
        } else if is_dir {
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