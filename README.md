# struct

i got tired of `tree` showing me 3000 files inside `node_modules` and `venv` every single time, so i made this.

## what it does

it's basically `tree` but it doesn't spam your terminal with garbage you don't care about.

```bash
# instead of this nightmare:
$ tree -L 3
venv/
├── lib/
│   ├── python3.11/
│   │   ├── site-packages/
│   │   │   ├── pip/
│   │   │   │   ├── __init__.py
│   │   │   │   ├── (2000 MORE FILES YOU DON'T CARE ABOUT)

# you get this:
$ struct 3
venv/ (2741 files ignored)
```

way better.

## install

```bash
cargo build --release
sudo cp target/release/struct /usr/local/bin/
```

## usage

```bash
struct 3              # depth 3 (like tree -L 3)
struct -g 2           # only git-tracked files
struct -s 100 3       # skip folders bigger than 100MB
struct -i "*.log" 2   # custom ignores
```

## what gets auto-ignored

the usual suspects:
- `venv`, `node_modules` (shows folder but not the 10000 files inside)
- `__pycache__`, `.git`, `target`
- `.vscode`, `.idea`
- `chrome_profile` and other cache garbage
- basically anything that clutters your output

folders still show up, you just see `venv/ (2741 files ignored)` instead of everything exploding.

## why rust

i wanted to learn rust and this seemed like a good starter project. also it's fast.

## stuff it does

- colors: blue folders, green executables
- file counts next to ignored dirs
- git mode to see only tracked files
- size limits to skip massive folders
- custom ignore patterns

that's it. it's just `tree` but less annoying.

---

if you find bugs or want features, open an issue or whatever