# struct

A Rust-based tree alternative that actually respects your sanity.

## The Problem

Running `tree` in a project directory gives you this:

```bash
$ tree -L 3
venv/
├── lib/
│   ├── python3.11/
│   │   ├── site-packages/
│   │   │   ├── pip/
│   │   │   │   ├── __init__.py
│   │   │   │   ├── ... (2000+ files you didn't ask for)
```

I needed something that shows project structure without drowning me in dependency folders.

## What This Does

`struct` shows your project's actual structure while automatically hiding the noise:

```bash
$ struct 3
venv/ (2741 files ignored)
src/
├── main.rs
└── lib.rs
```

The folder still appears, but you get a clean file count instead of thousands of irrelevant paths.

## Installation

```bash
cargo build --release
sudo cp target/release/struct /usr/local/bin/
```

## Quick Start

```bash
struct 3                        # Show 3 levels deep (default)
struct 0                        # Show everything (infinite depth)
struct -z 2                     # Show with file sizes
struct search "*.py"            # Find all Python files
```

---

## Complete Usage Guide

### Basic Tree Display

**Show directory structure with depth limit:**

```bash
struct [DEPTH] [PATH]
```

- `DEPTH`: How many levels to show (default: 3, use 0 for infinite)
- `PATH`: Directory to display (default: current directory)

**Examples:**
```bash
struct                          # Current dir, 3 levels deep
struct 5                        # Current dir, 5 levels deep
struct 0                        # Current dir, unlimited depth
struct 2 ~/projects             # Projects folder, 2 levels deep
struct 0 /etc                   # All of /etc
```

---

### Flags and Options

#### `-z, --size`
Show file sizes for all files and ignored directories.

```bash
struct -z 3                     # Show sizes
struct --size 2                 # Long form
```

**Output:**
```
main.rs (8.5K)
venv/ (156.3M, 2741 files ignored)
```

#### `-g, --git`
Show only git-tracked files (ignores everything not in git).

```bash
struct -g 2                     # Git-tracked files only
struct --git 3                  # Long form
```

**Use case:** Clean view of actual source code without build artifacts.

#### `-s, --skip-large SIZE_MB`
Skip folders larger than specified size in megabytes.

```bash
struct -s 100 3                 # Skip folders > 100MB
struct --skip-large 500 2       # Skip folders > 500MB
```

**Output:**
```
node_modules/ (450MB, skipped)
```

#### `-i, --ignore PATTERNS`
Add custom ignore patterns (comma-separated, wildcards supported).

```bash
struct -i "*.log" 3             # Ignore .log files
struct -i "*.tmp,cache*" 2      # Multiple patterns
struct --ignore "test*,*.bak" 3 # Long form
```

#### `-n, --no-ignore MODE`
Disable ignores selectively. MODE can be:
- `all` - Disable ALL ignores (show everything)
- `defaults` - Disable built-in defaults (venv, node_modules, etc.)
- `config` - Disable config file patterns only
- `PATTERN` - Show specific folder (e.g., `venv`, `node_modules`)

```bash
struct -n all 2                 # Show absolutely everything
struct -n defaults 3            # Show venv, __pycache__, etc.
struct -n config 2              # Ignore defaults but not config
struct -n venv 2                # Show venv contents only
struct -n node_modules 1        # Peek inside node_modules
struct --no-ignore all 3        # Long form
```

**Combining flags:**
```bash
struct -z -g 3                  # Git-tracked files with sizes
struct -n all -z 2              # Everything with sizes
struct -s 200 -i "*.log" 3      # Skip large + ignore logs
```

---

### Config File Management

Save ignore patterns permanently instead of typing `-i` every time.

**Location:** `~/.config/struct/ignores.txt`

#### `struct add PATTERN`
Add a pattern to permanent ignores.

```bash
struct add "chrome_profile"     # Add folder
struct add "*.log"              # Add file pattern
struct add "cache"              # Add another pattern
```

#### `struct remove PATTERN`
Remove a pattern from config.

```bash
struct remove "cache"           # Remove specific pattern
```

#### `struct list`
Show all saved patterns.

```bash
struct list
```

**Output:**
```
custom ignore patterns:
  chrome_profile
  *.log
  temp*

config file: /home/user/.config/struct/ignores.txt
```

#### `struct clear`
Delete all custom patterns.

```bash
struct clear
```

---

### Search

Find files by pattern across your project.

```bash
struct search PATTERN [OPTIONS] [PATH]
```

**Basic search:**
```bash
struct search "*.py"                    # All Python files (current dir)
struct search "*.env" ~/projects        # All .env files in ~/projects
struct search "config*"                 # Files starting with "config"
struct search "test*.rs" /code          # Rust test files in /code
```

**Search options:**

#### `-d, --depth DEPTH`
Limit search depth (default: 0 = infinite).

```bash
struct search "*.py" -d 2               # Only 2 levels deep
struct search "*.toml" --depth 1        # Top level only
struct search "*.js" -d 3 ~/code        # 3 levels in ~/code
```

#### `-f, --flat`
Show flat list of full paths instead of tree.

```bash
struct search "*.env" -f                # Flat output
struct search "*.py" --flat             # Long form
```

**Tree output (default):**
```
found 12 file(s) matching *.py

01_python/
├── calculator/
│   └── KalQl8er.py (24.4K)
├── bgm/
│   └── BGM.py (44.5K)
└── timebomb/
    └── timebomb.py (5.7K)
```

**Flat output (`-f`):**
```
found 12 file(s) matching *.py

/home/user/projects/01_python/calculator/KalQl8er.py (24.4K)
/home/user/projects/01_python/bgm/BGM.py (44.5K)
/home/user/projects/01_python/timebomb/timebomb.py (5.7K)
```

**Combining search options:**
```bash
struct search "*.rs" -d 2 -f            # Rust files, 2 levels, flat
struct search "test*" --depth 1 --flat ~/code  # Top-level tests, flat
```

---

## Auto-Ignored Directories

These are hidden by default (folder shown with file count):

**Python:**
- `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`
- `*.pyc`, `*.pyo`, `*.pyd` files
- `*.egg-info`, `dist`, `build`, `.tox`
- `venv`, `.venv`, `env`, `virtualenv`

**JavaScript/Node:**
- `node_modules`, `.npm`, `.yarn`

**Version Control:**
- `.git`, `.svn`, `.hg`

**IDEs/Editors:**
- `.vscode`, `.idea`, `.obsidian`
- `*.swp`, `*.swo` files

**Build Artifacts:**
- `target` (Rust/Java)
- `bin`, `obj` (C#)
- `.next`, `.nuxt` (JS frameworks)

**Caches:**
- `chrome_profile`, `lofi_chrome_profile`
- `GPUCache`, `ShaderCache`, `GrShaderCache`
- `Cache`, `blob_storage`

**Other:**
- `.DS_Store` (macOS)

Use `-n all` to show everything, or `-n PATTERN` to show specific folders.

---

## Features

- **Color-coded output**: Directories in blue, executables in green
- **File counts**: Shows how many files are being hidden
- **Git integration**: Filter to only git-tracked files
- **Size awareness**: Skip folders over a certain size
- **Configurable**: Save your ignore patterns permanently
- **Fast search**: Find files with pattern matching
- **Flexible output**: Tree or flat format

---

## Real-World Examples

**Check project structure without clutter:**
```bash
cd ~/myproject
struct 3
```

**Find all config files:**
```bash
struct search "*.env"
struct search "config*" -d 2
```

**See what's actually tracked in git:**
```bash
struct -g 2
```

**Peek inside an ignored folder:**
```bash
struct -n venv 2
struct -n node_modules 1
```

**Find large folders:**
```bash
struct -z 2                     # Show all sizes
struct -s 100 3                 # Skip folders > 100MB
```

**Search with flat output for grep/scripting:**
```bash
struct search "*.py" -f | grep test
```

---

## Why Rust

This started as a learning project to get hands-on with Rust. Turned out to be genuinely useful, so I polished it up. The performance is a nice bonus.

## Contributing

Found a bug? Want a feature? Open an issue. PRs welcome.

## License

MIT