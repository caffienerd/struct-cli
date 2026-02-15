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

### Option 1: Install from crates.io
The easiest way is to install directly via Cargo (make sure you have Rust installed):

```bash
cargo install struct-cli
```

View on [crates.io](https://crates.io/crates/struct-cli)

### Option 2: Install from source
```
git clone https://github.com/caffienerd/struct-cli.git
cd struct-cli
chmod +x install.sh && ./install.sh
```

## Uninstallation

```bash
git clone https://github.com/caffienerd/struct-cli.git && cd struct-cli
chmod +x uninstall.sh && ./uninstall.sh
```

## Quick Start

```bash
struct                          # Show everything (infinite depth by default)
struct 0                        # Show detailed summary of current directory
struct 3                        # Show 3 levels deep
struct 5 -z                     # Show 5 levels with file sizes
struct 3 -p ~/projects          # Show ~/projects, 3 levels deep
```

---

## Complete Usage Guide

### struct 0 - Directory Summary Mode

When you run `struct 0`, you get a detailed summary of the current directory with stats for each item:

```bash
struct 0
```

**Output:**
```
/home/user/projects/myproject (main)

src/
  /home/user/projects/myproject/src
  total:    10 dirs · 45 files · 125.3K
  visible:  8 dirs · 42 files · 120.1K
  types:    rs(30) toml(5) md(3) json(2) txt(2)
  ignored:  target(948 files)

README.md
  /home/user/projects/myproject/README.md
  12.5K

.gitignore
  /home/user/projects/myproject/.gitignore
  486B

── ignored (top level) ──
  .git(60 files), target(948 files) · 1008 files · 45.2M
```

**What it shows:**
- Current directory path with git branch
- For each directory:
  - Full path
  - Total stats (all files recursively)
  - Visible stats (excluding ignored folders)
  - File type breakdown
  - Ignored subdirectories
- For each file:
  - Full path
  - File size
- Summary of top-level ignored items

**Use cases:**
- Quick directory analysis
- Find what's taking up space
- See project composition at a glance
- Identify ignored bloat

---

### Basic Tree Display

**Show directory structure with depth limit:**

```bash
struct [DEPTH] [OPTIONS]
```

- `DEPTH`: How many levels to show (default: infinite, 0 = current dir only)
- Use `-p` or `--path` to specify a different directory

**Examples:**
```bash
struct                          # Current dir, infinite depth
struct 0                        # Current dir only (1 level)
struct 3                        # Current dir, 3 levels deep
struct 5 -p ~/projects          # Projects folder, 5 levels
struct 2 --path /etc            # /etc, 2 levels
```

---

### Flags and Options

#### `-z, --size`
Show file sizes for all files and ignored directories.

```bash
struct 3 -z                     # Show sizes
struct 2 --size                 # Long form
```

**Output:**
```
main.rs (8.5K)
venv/ (156.3M, 2741 files ignored)
```

#### `-p, --path PATH`
Specify directory to display (default: current directory).

```bash
struct 3 -p ~/projects          # Projects folder, 3 levels
struct --path /etc              # /etc directory
struct 5 -p ~/code -z           # Code folder with sizes
```

#### `-g, --git`
Show only git-tracked files (ignores everything not in git).

```bash
struct 2 -g                     # Git-tracked files only
struct 3 --git                  # Long form
```

**Use case:** Clean view of actual source code without build artifacts.

#### `-s, --skip-large SIZE_MB`
Skip folders larger than specified size in megabytes.

```bash
struct 3 -s 100                 # Skip folders > 100MB
struct 2 --skip-large 500       # Skip folders > 500MB
```

**Output:**
```
node_modules/ (450MB, skipped)
```

#### `-i, --ignore PATTERNS`
Add custom ignore patterns (comma-separated, wildcards supported).

```bash
struct 3 -i "*.log"             # Ignore .log files
struct 2 -i "*.tmp,cache*"      # Multiple patterns
struct 3 --ignore "test*,*.bak" # Long form
```

#### `-n, --no-ignore MODE`
Disable ignores selectively. MODE can be:
- `all` - Disable ALL ignores (show everything)
- `defaults` - Disable built-in defaults (venv, node_modules, etc.)
- `config` - Disable config file patterns only
- `PATTERN` - Show specific folder (e.g., `venv`, `node_modules`)

```bash
struct 2 -n all                 # Show absolutely everything
struct 3 -n defaults            # Show venv, __pycache__, etc.
struct 2 -n config              # Ignore defaults but not config
struct 2 -n venv                # Show venv contents only
struct 1 -n node_modules        # Peek inside node_modules
struct 3 --no-ignore all        # Long form
```

**Combining flags:**
```bash
struct 3 -z -g                  # Git-tracked files with sizes
struct 2 -n all -z              # Everything with sizes
struct 3 -s 200 -i "*.log"      # Skip large + ignore logs
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
struct 2 -g
```

**Peek inside an ignored folder:**
```bash
struct 2 -n venv
struct 1 -n node_modules
```

**Find large folders:**
```bash
struct 2 -z                     # Show all sizes
struct 3 -s 100                 # Skip folders > 100MB
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

MIT - feel free to do whatever you want with it!