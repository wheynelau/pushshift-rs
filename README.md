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

- If an array of files is provided, the filtering is done in parallel using rayon. There is also an experimental flag called `--multithreaded`,
however it is meant to split a file into two workers. To reduce resource contention, its better to use the defaults. 

Worker 1: Handles read and decompress
Worker 2: Json serialization, filtering and writing

- `zstd` supports multithreaded compression, but there are not plans for implementation. 

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

Each line of json will be a single post, with DFS order on the comments. Additionally, there is a flag `--include-scores`, that adds the scores to the comments.
It can be useful if you need the scores information for a downstream task. 

### Example of an output

The output always contains three fields, `raw_content`, `subreddit`, and `length`. 

For now the length is just a simple `.len()`, but can be modified to use tokenizers or split by whitespace. 

`raw_content` is the text of the DFS processing.

```json
{
    "raw_content": "What are you waiting for?\n\n{score: 2 Ups: 2 Downs: 0}\n\nyup!\n\n{score: 3 Ups: 3 Downs: 0}",
    "subreddit" : "funny",
    "length" : 600
}
```

## Issues and Contributions

This project is mostly for my own use cases, but if you find that some feature may help other users, please raise an issue or a PR. 