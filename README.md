# reddit-rs

> **Note:** This is a personal project and may not be actively maintained. Feel free to extend it for your own use cases.

A CLI tool for processing and filtering Reddit data from Pushshift archives (historical Reddit posts and comments in compressed JSON format).

## Installation

Binaries available on the [releases page](https://github.com/wheynelau/pushshift-rs/releases).

Download the appropriate binary for your platform and extract it. Either add the directory to your PATH or run the executable directly.

Only Windows ARM is not available.

## Data source

Pushshift reddit archives available on [AcademicTorrents](https://academictorrents.com).

## Quickstart

Basic workflow for filtering and processing subreddits:

```bash
# 1. Filter submissions (RS = Reddit Submissions)
reddit-rs filter -i RS_2025-*.zst -n funny pics -o filtered_{basename}.zst

# 2. Filter comments (RC = Reddit Comments)
reddit-rs filter -i RC_2025-*.zst -n funny pics -o filtered_{basename}.zst

# 3. Process into conversation threads
reddit-rs process -s filtered_RS_2025-*.zst -c filtered_RC_2025-*.zst -o threads.zst
```

Submissions and comments must be filtered separately before processing.

## Data format

Input files contain one JSON object per line. The main fields used by this tool:

**Comments:**
- `subreddit` - subreddit name (used for filtering)
- `link_id` - submission the comment belongs to (`t3_*`)
- `parent_id` - what this comment replies to (`t3_*` for submission, `t1_*` for another comment)
- `body` - comment text
- `author` - username

**Submissions:**
- `subreddit` - subreddit name (used for filtering)
- `id` - submission ID
- `title` - post title
- `selftext` - post body (for text posts)
- `author` - username
- `url` - link URL

## Filter

Filter should be your first step. Use it when you only want specific subreddits from the archive.

Input accepts either posts or comments formats.

Output must end with `.jsonl` or `.zst` (the extension is required and validated). Use `.zst` for better compression ratios, unless you have filesystem-level compression like `btrfs`.

**Important:** If your output path includes directories (e.g., `output/filtered.zst`), those directories must exist before running the command. They won't be created automatically as a safety measure to prevent accidental writes to unintended locations.

### Arguments

- `-i, --input` - Input file(s), supports glob patterns like `*.zst`
- `-n, --names` - Subreddit names to filter (space-separated)
- `-o, --output` - Output file pattern. Supports `{basename}` and `{subreddit}` placeholders
- `-l, --level` - Compression level for `.zst` output (default: 3, range 1-22). Higher levels = better compression but slower. Level 3 balances speed and size. Pushshift archives use level 22 for maximum compression
- `--workers` - Number of parallel worker threads for `zstd` compression (default: 0, single-threaded). Set to 1 or higher to enable multithreading
- `--split` - Create separate output file for each subreddit (requires `{subreddit}` in output pattern)

### Limitations

The `--split` flag only works with subreddits you explicitly specify via `-n`. It won't automatically discover and split by all subreddits in the input. Each subreddit must be listed.

### Example

**Single output file (default):**
```bash
reddit-rs filter -i *.zst -n funny wallstreetbets -o {basename}_filtered.zst -l 3
```
Filters from `funny` and `wallstreetbets` into combined files (one per input file).

**Split by subreddit:**
```bash
reddit-rs filter -i RC_2025-*.zst -n funny pics --split -o {subreddit}_{basename}_filtered.zst
```
Creates separate files per subreddit:
- `funny_RC_2025-01_filtered.zst`
- `funny_RC_2025-02_filtered.zst`
- `pics_RC_2025-01_filtered.zst`
- `pics_RC_2025-02_filtered.zst`

Placeholders:
- `{basename}` - Original input filename without extension
- `{subreddit}` - Subreddit name (required when using `--split`)

### Performance

Multiple input files are processed in parallel using rayon. Use `--workers` to control `zstd` compression threads.

### Note

The filter only checks for the `subreddit` field. It works on any dataset with this field, not just Pushshift data.

## Process

Merges posts and comments into linear conversation branches. Useful for LLM training data.

### Arguments

- `-s, --submissions` - Submission/post files, supports glob patterns
- `-c, --comments` - Comment files, supports glob patterns
- `-o, --output` - Output file (must end with `.jsonl` or `.zst`)

### Example

```bash
reddit-rs process -s submissions_*.zst -c comments_*.zst -o output.zst
```

### How it works

A linear branch is a chain of comments where each comment replies to the previous one, starting from a top-level comment.

For example, given this reply structure:
- A (top-level comment)
  - B (replies to A)
    - C (replies to B)
    - D (replies to B)
  - E (replies to A)

The output contains three branches:
1. A → B → C
2. A → B → D
3. A → E

Each branch is one JSON object per line, containing the full conversation thread. This provides sequential context for language models.

**Output format:**
```json
{
  "raw_content": "# Post\n\nOriginal post title from r/example:\n\"Post title here\"\n\n---\n\n## Reply\n\nComment text",
  "length": 214,
  "subreddit": "example",
  "permalink": "/r/example/comments/...",
  "created_utc": 1735716551
}
```

**Note:** This is experimental. The linear branch approach maintains a single conversational flow per thread, which may help LLM training (though this is unproven). The trade-off is that heavily-branching conversations get split into multiple shorter threads.

## Issues and Contributions

This project is mostly for my own use cases, but if you find that some feature may help other users, please raise an issue or a PR. 