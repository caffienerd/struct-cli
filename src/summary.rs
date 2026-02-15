use colored::*;
use git2::Repository;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::load_config_patterns;
use crate::ignores::{should_ignore_dir, should_ignore_file, matches_custom_pattern};
use crate::utils::{format_size, get_dir_size, is_executable};

/// Display detailed summary of current directory (struct 0 mode)
pub fn display_summary(path: &Path) {
    // Get absolute path
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    
    // Show current directory header with git branch if available
    let mut header = format!("{}", abs_path.display());
    if let Ok(repo) = Repository::discover(path) {
        if let Ok(head) = repo.head() {
            if let Some(branch) = head.shorthand() {
                header = format!("{} {}", abs_path.display(), format!("({})", branch).bright_black().to_string());
            }
        }
    }
    println!("{}", header.cyan().bold());
    println!();

    let entries: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            eprintln!("failed to read directory: {}", e);
            return;
        }
    };

    // Load config patterns for filtering
    let config_patterns = load_config_patterns();
    let mut custom_ignores = Vec::new();
    for pattern in config_patterns {
        let pattern = pattern.replace("*", ".*");
        if let Ok(re) = Regex::new(&format!("^{}$", pattern)) {
            custom_ignores.push(re);
        }
    }

    let mut total_ignored_files = 0;
    let mut total_ignored_size = 0u64;
    let mut ignored_names = Vec::new();

    for entry in entries {
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry_path.is_dir();

        // Check if should be ignored
        let should_skip = if is_dir {
            should_ignore_dir(&name) || matches_custom_pattern(&name, &custom_ignores)
        } else {
            should_ignore_file(&name) || matches_custom_pattern(&name, &custom_ignores)
        };

        if should_skip {
            // Track ignored items
            if is_dir {
                let file_count = WalkDir::new(&entry_path)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .count();
                let size = get_dir_size(&entry_path);
                total_ignored_files += file_count;
                total_ignored_size += size;
                ignored_names.push(format!("{}({} files)", name, file_count));
            } else {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                total_ignored_files += 1;
                total_ignored_size += size;
                ignored_names.push(name);
            }
            continue;
        }

        if is_dir {
            display_directory_summary(&entry_path, &name, &custom_ignores);
        } else {
            display_file_summary(&entry_path, &name);
        }
    }

    // Show total ignored items summary at the end
    if total_ignored_files > 0 {
        println!("{}", "── ignored (top level) ──".bright_black());
        println!("  {} · {} · {}", 
            ignored_names.join(", ").bright_black(),
            format!("{} files", total_ignored_files).bright_black(),
            format_size(total_ignored_size).bright_black()
        );
    }
}

fn display_directory_summary(entry_path: &Path, name: &str, custom_ignores: &[Regex]) {
    let mut total_file_count = 0;
    let mut total_dir_count = 0;
    let mut total_size: u64 = 0;

    let mut visible_file_count = 0;
    let mut visible_dir_count = 0;
    let mut visible_size: u64 = 0;
    let mut visible_extensions: HashMap<String, usize> = HashMap::new();
    let mut ignored_subdirs: Vec<(String, usize)> = Vec::new();

    // First, check immediate children for ignored subdirs
    if let Ok(immediate_entries) = fs::read_dir(entry_path) {
        for immediate in immediate_entries.filter_map(|e| e.ok()) {
            let subname = immediate.file_name().to_string_lossy().to_string();
            let subpath = immediate.path();
            let is_subdir = subpath.is_dir();

            if is_subdir && (should_ignore_dir(&subname) || matches_custom_pattern(&subname, custom_ignores)) {
                // Count files in ignored subdir
                let ignored_count = WalkDir::new(&subpath)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .count();
                ignored_subdirs.push((subname, ignored_count));
            }
        }
    }

    // Walk recursively to count visible items (skip ignored directories)
    for sub_entry in WalkDir::new(entry_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip ignored directories during traversal
            if e.file_type().is_dir() && e.path() != entry_path {
                if let Some(name) = e.file_name().to_str() {
                    return !(should_ignore_dir(name) || matches_custom_pattern(name, custom_ignores));
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        let subpath = sub_entry.path();
        let subname = sub_entry.file_name().to_string_lossy().to_string();

        if sub_entry.file_type().is_file() {
            // Check if file itself should be ignored
            if !should_ignore_file(&subname) && !matches_custom_pattern(&subname, custom_ignores) {
                visible_file_count += 1;
                if let Ok(metadata) = sub_entry.metadata() {
                    visible_size += metadata.len();
                }
                if let Some(ext) = subpath.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    *visible_extensions.entry(ext_str).or_insert(0) += 1;
                }
            }
        } else if sub_entry.file_type().is_dir() && subpath != entry_path {
            visible_dir_count += 1;
        }
    }

    // Get ALL stats recursively (including everything)
    for sub_entry in WalkDir::new(entry_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok()) {
        if sub_entry.file_type().is_file() {
            total_file_count += 1;
            if let Ok(metadata) = sub_entry.metadata() {
                total_size += metadata.len();
            }
        } else if sub_entry.file_type().is_dir() && sub_entry.path() != entry_path {
            total_dir_count += 1;
        }
    }

    // Display directory
    println!("{}", format!("{}/", name).blue().bold());
    println!("  {}", entry_path.canonicalize().unwrap_or(entry_path.to_path_buf()).display().to_string().bright_black());
    
    // Check if visible is different from total
    let has_ignored = visible_dir_count < total_dir_count || 
                      visible_file_count < total_file_count ||
                      visible_size < total_size;
    
    if has_ignored {
        // Show both total and visible
        let total_parts = vec![
            format!("{} dirs", total_dir_count),
            format!("{} files", total_file_count),
            format_size(total_size).to_string()
        ];
        println!("  {:<9} {}", "total:".bright_black(), total_parts.join(" · ").yellow());

        let mut visible_parts = Vec::new();
        if visible_dir_count > 0 {
            visible_parts.push(format!("{} dirs", visible_dir_count));
        }
        if visible_file_count > 0 {
            visible_parts.push(format!("{} files", visible_file_count));
        }
        visible_parts.push(format_size(visible_size).to_string());
        println!("  {:<9} {}", "visible:".bright_black(), visible_parts.join(" · ").green());
    } else {
        // Just show total (since visible = total)
        let mut parts = Vec::new();
        if total_dir_count > 0 {
            parts.push(format!("{} dirs", total_dir_count));
        }
        if total_file_count > 0 {
            parts.push(format!("{} files", total_file_count));
        }
        parts.push(format_size(total_size).to_string());
        println!("  {:<9} {}", "total:".bright_black(), parts.join(" · ").yellow());
    }

    // Types line (from visible files)
    if !visible_extensions.is_empty() {
        let mut ext_vec: Vec<_> = visible_extensions.iter().collect();
        ext_vec.sort_by(|a, b| b.1.cmp(a.1));
        let type_summary: Vec<String> = ext_vec.iter()
            .take(10)
            .map(|(ext, count)| format!("{}({})", ext, count))
            .collect();
        println!("  {:<9} {}", "types:".bright_black(), type_summary.join(" ").cyan());
    }

    // Ignored subdirs
    if !ignored_subdirs.is_empty() {
        let ignored_str: Vec<String> = ignored_subdirs.iter()
            .map(|(name, count)| format!("{}({} files)", name, count))
            .collect();
        println!("  {:<9} {}", "ignored:".bright_black(), ignored_str.join(", ").bright_black());
    }

    println!();
}

fn display_file_summary(entry_path: &Path, name: &str) {
    let size = entry_path.metadata().map(|m| m.len()).unwrap_or(0);
    let display_name = if is_executable(entry_path) {
        name.green().bold()
    } else {
        name.normal()
    };
    
    println!("{}", display_name);
    println!("  {}", entry_path.canonicalize().unwrap_or(entry_path.to_path_buf()).display().to_string().bright_black());
    println!("  {}", format_size(size).bright_black());
    println!();
}