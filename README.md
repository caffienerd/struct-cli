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

## Usage

```bash
struct 3              # Show structure up to depth 3
struct 0              # Show everything (infinite depth)
struct --size 2       # Show file sizes
struct -g 2           # Git-tracked files only
struct -s 100 3       # Skip folders larger than 100MB
struct -i "*.log" 2   # Add custom ignore patterns
```

### Config File

Save patterns permanently instead of retyping them:

```bash
struct add "chrome_profile"     # Add to permanent ignores
struct add "*.log"              # Wildcards supported
struct list                     # View saved patterns
struct remove "cache"           # Remove specific pattern
struct clear                    # Reset config
```

Config is stored in `~/.config/struct/ignores.txt`

## Auto-Ignored Directories

Common bloat folders are hidden by default:
- Python: `venv`, `__pycache__`, `dist`, `build`, `.pytest_cache`
- Node: `node_modules`, `.npm`, `.yarn`
- Version Control: `.git`, `.svn`, `.hg`
- IDEs: `.vscode`, `.idea`, `.obsidian`
- Build artifacts: `target`, `bin`, `obj`
- Caches: `chrome_profile`, `GPUCache`, `ShaderCache`

## Features

- **Color-coded output**: Directories in blue, executables in green
- **File counts**: Shows how many files are being hidden
- **Git integration**: Filter to only git-tracked files
- **Size awareness**: Skip folders over a certain size
- **Configurable**: Save your ignore patterns permanently

## Why Rust

This started as a learning project to get hands-on with Rust. Turned out to be genuinely useful, so I polished it up. The performance is a nice bonus.

## Contributing

Found a bug? Want a feature? Open an issue. PRs welcome.

## License

MIT