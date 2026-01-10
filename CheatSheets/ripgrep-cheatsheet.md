# ripgrep (rg) Cheat Sheet

## Installation

```bash
# Ubuntu/WSL
sudo apt update
sudo apt install ripgrep

# Verify
rg --version
```

---

## Basic Usage

```bash
rg "pattern"                    # Basic search
rg "pattern" path/              # Search in specific path
rg "pattern" file.rs            # Search in specific file
rg --help                       # Show help
```

---

## Case Sensitivity

```bash
rg "pattern"                    # Case-sensitive (default)
rg -i "pattern"                 # Case-insensitive
rg --ignore-case "pattern"      # Case-insensitive (long form)
rg -S "pattern"                 # Smart case (insensitive if lowercase)
rg --smart-case "pattern"       # Smart case (long form)
```

---

## Pattern Matching

```bash
rg "pattern"                    # Regex search (default)
rg -w "word"                    # Whole word only
rg --word-regexp "word"         # Whole word (long form)
rg -F "string"                  # Fixed string (literal, no regex)
rg --fixed-strings "string"     # Fixed string (long form)
rg -e "pattern1" -e "pattern2"  # Multiple patterns (OR)
```

---

## Context Lines

```bash
rg "pattern" -A 2               # Show 2 lines AFTER match
rg "pattern" --after-context 2  # Show lines after (long form)
rg "pattern" -B 2               # Show 2 lines BEFORE match
rg "pattern" --before-context 2 # Show lines before (long form)
rg "pattern" -C 2               # Show 2 lines BEFORE and AFTER
rg "pattern" --context 2        # Show context (long form)
```

**Example:**
```bash
rg "fn add_order" -A 10         # See function + next 10 lines
rg "TODO" -C 3                  # See TODO with 3 lines context
```

---

## Output Control

```bash
rg "pattern" -n                 # Show line numbers (default)
rg "pattern" --line-number      # Show line numbers (long form)
rg "pattern" -N                 # Hide line numbers
rg "pattern" --no-line-number   # Hide line numbers (long form)
rg "pattern" -l                 # Show only filenames with matches
rg "pattern" --files-with-matches  # Filenames only (long form)
rg "pattern" --files-without-match # Files WITHOUT matches
rg "pattern" -c                 # Count matches per file
rg "pattern" --count            # Count (long form)
rg "pattern" -o                 # Show only matching part
rg "pattern" --only-matching    # Only matching (long form)
rg "pattern" --column           # Show column numbers
rg "pattern" -q                 # Quiet (suppress output)
rg "pattern" --quiet            # Quiet (long form)
```

---

## File Type Filtering

```bash
rg "pattern" -t rust            # Search only Rust files
rg "pattern" --type rust        # Type filter (long form)
rg "pattern" -T rust            # Exclude Rust files
rg "pattern" --type-not rust    # Exclude type (long form)
rg --type-list                  # List all available types

# Common types:
rg "pattern" -t rust            # .rs files
rg "pattern" -t python          # .py files
rg "pattern" -t js              # .js, .jsx, .mjs files
rg "pattern" -t json            # .json files
rg "pattern" -t yaml            # .yaml, .yml files
rg "pattern" -t toml            # .toml files
rg "pattern" -t md              # .md, .markdown files
rg "pattern" -t c               # .c, .h files
rg "pattern" -t cpp             # .cpp, .cc, .hpp files
```

---

## Glob Patterns

```bash
rg "pattern" -g "*.rs"          # Search only .rs files
rg "pattern" --glob "*.rs"      # Glob (long form)
rg "pattern" -g "*.{rs,toml}"   # Multiple extensions
rg "pattern" -g "!*.test.rs"    # Exclude test files
rg "pattern" -g "!test*"        # Exclude test* pattern
rg "pattern" -g "src/**/*.rs"   # Recursive in src/
```

**Examples:**
```bash
# Search Rust files in src/, exclude tests
rg "pattern" -g "src/**/*.rs" -g "!**/*test*"

# Search only in specific directories
rg "pattern" -g "src/*" -g "tests/*"
```

---

## Hidden & Ignored Files

```bash
rg "pattern" -.                 # Include hidden files (.gitignore, .env)
rg "pattern" --hidden           # Hidden (long form)
rg "pattern" --no-ignore        # Don't respect .gitignore
rg "pattern" --no-ignore --hidden  # Both
rg "pattern" -u                 # Same as --no-ignore
rg "pattern" -uu                # --no-ignore --hidden
rg "pattern" -uuu               # --no-ignore --hidden --binary
```

**Common use:**
```bash
# Search everything except target/ and .git/
rg "pattern" --hidden --no-ignore -g "!target/" -g "!.git/"
```

---

## Limiting Results

```bash
rg "pattern" -m 5               # Max 5 matches per file
rg "pattern" --max-count 5      # Max count (long form)
rg "pattern" --max-depth 2      # Limit directory depth
```

---

## Multiline Matching

```bash
rg "pattern.*\n.*other" -U      # Multiline mode
rg "pattern" --multiline        # Multiline (long form)
```

**Example:**
```bash
# Find struct with fields across lines
rg "struct OrderBook \{.*symbol.*\}" -U
```

---

## Replacement (Preview Only)

```bash
rg "old" -r "new"               # Show replacement preview
rg "old" --replace "new"        # Replace (long form)
rg "fn (\w+)" -r "function $1"  # With capture groups
```

**Note:** `rg` doesn't modify files. Use `sd` for actual replacements.

---

## Performance & Sorting

```bash
rg "pattern" -j 4               # Use 4 threads
rg "pattern" --threads 4        # Threads (long form)
rg "pattern" --mmap             # Use memory mapping (faster)
rg "pattern" --sort path        # Sort by file path
rg "pattern" --sort modified    # Sort by modification time
rg "pattern" --sort accessed    # Sort by access time
rg "pattern" --sort created     # Sort by creation time
```

---

## Color & Formatting

```bash
rg "pattern" --color always     # Force color output
rg "pattern" --color never      # Disable color
rg "pattern" --color auto       # Auto (default)
rg "pattern" --no-heading       # Compact output
rg "pattern" --heading          # Group by file (default)
```

---

## Special Modes

```bash
rg "pattern" --stats            # Show search statistics
rg "pattern" --json             # Output as JSON
rg "pattern" -P                 # Use PCRE2 regex engine
rg "pattern" --pcre2            # PCRE2 (long form)
rg --files                      # List files that would be searched
rg "pattern" --debug            # Debug mode (why files ignored)
```

---

## Rust-Specific Searches

### Find Structs
```bash
rg "^pub struct" -t rust
rg "^struct \w+" -t rust
rg "^pub struct OrderBook" -A 15 -t rust
```

### Find Enums
```bash
rg "^pub enum" -t rust
rg "^enum \w+" -t rust
```

### Find Traits
```bash
rg "^pub trait" -t rust
rg "^trait \w+" -t rust
```

### Find Implementations
```bash
rg "^impl" -t rust
rg "impl.*for" -t rust
rg "impl OrderBook" -A 10 -t rust
```

### Find Functions
```bash
rg "^pub fn" -t rust
rg "^fn \w+" -t rust
rg "fn add_order" -A 20 -t rust
```

### Find Methods
```bash
rg "\.method_name\(" -t rust
rg "order_book\." -t rust
```

### Find TODOs
```bash
rg "TODO|FIXME|XXX|HACK" -i -t rust
```

### Find Panics/Unwraps
```bash
rg "panic!|unwrap\(\)|expect\(" -t rust
```

### Find Async Functions
```bash
rg "async fn" -t rust
```

### Find Macros
```bash
rg "macro_rules!" -t rust
rg "#\[derive" -t rust
```

---

## Common Combinations

```bash
# Find struct with full definition and context
rg "^pub struct OrderBook" -A 20 -t rust

# Count all TODO comments
rg "TODO" -c -t rust

# Find all uses of a type (whole word)
rg -w "OrderBook" -t rust

# Search only in src/, exclude tests
rg "pattern" -g "src/**/*.rs" -g "!**/*test*"

# Case-insensitive with context
rg -i "error" -C 3 -t rust

# Find and show only filenames
rg "panic!" -l -t rust

# Multiline struct search
rg "struct OrderBook.*\n.*symbol" -U -t rust

# Search with statistics
rg "pattern" --stats -t rust
```

---

## Useful Aliases (Add to ~/.bashrc)

```bash
# Rust-specific searches
alias rgrs='rg -t rust'
alias rgfn='rg "^pub fn" -t rust'
alias rgst='rg "^pub struct" -t rust'
alias rgen='rg "^pub enum" -t rust'
alias rgtr='rg "^pub trait" -t rust'
alias rgim='rg "^impl" -t rust'
alias rgtodo='rg "TODO|FIXME|XXX" -i'
alias rgpanic='rg "panic!|unwrap\(\)|expect\("'

# Quick searches
alias rgl='rg -l'           # Files only
alias rgc='rg -c'           # Count per file
alias rgi='rg -i'           # Case-insensitive
alias rgw='rg -w'           # Whole word
alias rgA='rg -A 10'        # 10 lines after
alias rgC='rg -C 5'         # 5 lines context
```

---

## Configuration File

Create `~/.ripgreprc`:
```bash
# Default options
--smart-case
--hidden
--glob=!.git/*
--glob=!target/*
--glob=!node_modules/*
--glob=!*.lock
--max-columns=200
```

Enable it:
```bash
export RIPGREP_CONFIG_PATH=~/.ripgreprc
```

Add to `~/.bashrc` to make permanent.

---

## Integration with Other Tools

```bash
# Open matches in vim
vim $(rg "pattern" -l)

# Count total matches across all files
rg "pattern" -c | awk -F: '{sum+=$2} END {print sum}'

# Find and copy to clipboard (requires xclip)
rg "pattern" | xclip -selection clipboard

# Find and delete files (careful!)
rg "deprecated" -l | xargs rm

# Find and replace with sd
rg "old_name" -l | xargs sd "old_name" "new_name"

# Pipe to less with color
rg "pattern" --color always | less -R
```

---

## Quick Reference Table

| Task | Command |
|------|---------|
| Basic search | `rg "pattern"` |
| Case-insensitive | `rg -i "pattern"` |
| Show context | `rg "pattern" -C 3` |
| Rust files only | `rg "pattern" -t rust` |
| Files with matches | `rg "pattern" -l` |
| Count per file | `rg "pattern" -c` |
| Whole word | `rg -w "word"` |
| Literal string | `rg -F "string"` |
| Include hidden | `rg "pattern" --hidden` |
| Ignore .gitignore | `rg "pattern" --no-ignore` |
| Search everything | `rg "pattern" -uuu` |
| Multiline | `rg "pattern" -U` |
| Show stats | `rg "pattern" --stats` |

---

## Tips & Tricks

1. **Use smart-case by default**: Add `--smart-case` to `~/.ripgreprc`
2. **Combine flags**: `rg -i -w -C 3` or `rg -iwC3`
3. **Quote patterns**: Use `"pattern"` to avoid shell interpretation
4. **Escape regex**: Use `-F` for literal strings if you have special chars
5. **Use `-l` first**: Find files, then search in-depth with context
6. **Pipe to less**: `rg "pattern" --color always | less -R` for long output
7. **Custom types**: `rg --type-add 'web:*.{html,css,js}' -t web "pattern"`

---

## Common Pitfalls

❌ **Don't forget quotes**: `rg TODO` might not work, use `rg "TODO"`
❌ **Regex vs literal**: `rg "."` matches any char, use `rg -F "."` for literal dot
❌ **Case sensitivity**: Default is case-sensitive, use `-i` or `-S`
❌ **Hidden files**: `.env` won't be searched by default, use `--hidden`
❌ **Binary files**: Use `-uuu` to search binaries (usually not needed)

---

## See Also

- Official docs: https://github.com/BurntSushi/ripgrep
- Regex syntax: https://docs.rs/regex/latest/regex/#syntax
- User guide: https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md
