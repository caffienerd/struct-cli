use colored::*;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::ignores::{should_ignore_dir, matches_custom_pattern};
use crate::utils::{format_size, is_executable};

/// Search for files matching a pattern
pub fn search_files(pattern: &str, start_path: &Path, max_depth: usize, flat: bool, custom_ignores: &[Regex]) {
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

    // Search through all files and directories
    for entry in WalkDir::new(start_path)
        .follow_links(false)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| {
            // Skip common ignore directories to make search faster
            if let Some(name) = e.file_name().to_str() {
                !should_ignore_dir(name) && !matches_custom_pattern(name, custom_ignores)
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
    {
        if let Some(filename) = entry.file_name().to_str() {
            if re.is_match(filename) {
                let file_path = entry.path().to_path_buf();
                
                if flat {
                    // For flat output, just store path and size
                    let size = if entry.file_type().is_dir() {
                        0
                    } else {
                        entry.metadata().map(|m| m.len()).unwrap_or(0)
                    };
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

    if found_count == 0 {
        println!("{}", format!("no files or directories matching '{}' found", pattern).yellow());
        return;
    }

    println!("{} {}", format!("found {} item(s) matching", found_count).green(), pattern.cyan());
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
    _current_depth: usize,
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
            display_search_tree(&entry_path, matching_paths, 0, &new_prefix, is_last_entry);
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