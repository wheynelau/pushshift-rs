# reddit-rs

> **Note:** This is a personal project and may not be actively maintained. Feel free to extend it for your own use cases.

A CLI tool for processing and filtering Reddit data (Pushshift dumps).

## Installation

Binaries available on the [releases page](https://github.com/wheynelau/pushshift-rs/releases).

Only windows arm is not available. 

## Filter

Filter should be your first step, its used when you don't want to keep all the subreddits from the archive.

Input accepts either posts or comments formats. 

Output must end with `.jsonl` or `.zst`. `zst` is usually recommeneded, unless you have a drive with compression, such as `btrfs`.

Additionally, if your output specifies a folder, it is not created if it doesn't exist.

I am also not intending to implement a generic split by all subreddits. E.g `reddis-rs filter -i *.zst --split -o {basename}_{subreddit}_filtered.zst`,
the subreddit must be known at the filtering stage.

### Performance notes

- If an array of files is provided, the filtering is done in parallel using rayon. For `zstd` encoder, there is an argument `--workers` that allows for multithreaded `zstd` compression. 

### Example

```bash
reddis-rs filter -i *.zst -n funny wallstreetbets -o {basename}_filtered.zst -l 3
```

The above will filter all posts/comments from the subreddit `funny` and `wallstreetbets`, into multiple compressed files, with a compression level of 3.

### Post processing

Because all the filtering does is check for the field `subreddit`, you can technically use it for anything else as long as the field exists. For example you can filter after the process step below, or use it on other datasets with a `subreddit` field.

## Process

```bash
reddit-rs process -s submissions_*.zst -c comments_*.zst -o output.zst
```

### Notes
This task is mostly for LLM processing. It will merge the posts and comments into a single file.

A linear branch is defined as a chain of comments where each comment directly replies to the previous one, starting from a top-level comment (a comment that replies to the submission itself).

For example, if we have comments with the following reply structure:
- A (top-level comment)
  - B (replies to A)
    - C (replies to B)
    - D (replies to B)
  - E (replies to A)

The branches would be
1. A -> B -> C
2. A -> B -> D
3. A -> E

Each of these branches is represented as a single JSON object per line in the output file. This structure is particularly useful for language model processing, as it provides conversational context in a sequential format. This is still quite experimnetal, as the downside of this is potentially short threads.

## Issues and Contributions

This project is mostly for my own use cases, but if you find that some feature may help other users, please raise an issue or a PR. 