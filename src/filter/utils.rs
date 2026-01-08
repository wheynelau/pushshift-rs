use anyhow::{Context, Error, Result, bail};
use crossbeam_channel::bounded;
use gjson;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::FilterArgs;
use crate::common;
type FileMap = HashMap<String, Box<dyn Write>>;

pub fn validate_args(args: &FilterArgs) -> Result<()> {
    // 1. Use bail! for early returns. No more manual Error::new calls.

    let valid_placeholders = &["basename", "subreddit", "timestamp"];

    if let Some(output) = &args.output {
        let output_str = output.to_string_lossy();

        // 2. You can use .context() or .with_context() if this Regex ever fails
        let placeholder_re =
            Regex::new(r"\{([^}]+)\}").context("Failed to compile placeholder regex")?;

        let mut invalid_placeholders = Vec::new();

        for cap in placeholder_re.captures_iter(&output_str) {
            if let Some(placeholder) = cap.get(1) {
                let name = placeholder.as_str();
                if !valid_placeholders.contains(&name) {
                    invalid_placeholders.push(format!("{{{name}}}"));
                }
            }
        }
        // check if file ends with jsonl or zst
        if !output_str.ends_with(".jsonl") && !output_str.ends_with(".zst") {
            bail!("Output file must end with .jsonl or .zst");
        }

        if !invalid_placeholders.is_empty() {
            bail!(
                "Invalid placeholder(s): {}. Valid options are: {{basename}}, {{subreddit}}, {{timestamp}}",
                invalid_placeholders.join(", ")
            );
        }

        if args.split && !output_str.contains("{subreddit}") {
            bail!(
                "When using --split, --output must contain {{subreddit}} placeholder.\n\
                 Example: --output filtered_{{subreddit}}.jsonl"
            );
        }

        if args.input.len() > 1 && !output_str.contains("{basename}") {
            bail!(
                "When processing multiple input files, --output must contain {{basename}} placeholder to avoid concurrent writes.\n\
                 Example: --output processed/{{basename}}.zst"
            );
        }
    }

    Ok(())
}

/// Expands placeholders in a template string
fn expand_placeholders(
    template: &str,
    basename: &str,
    subreddit: Option<&str>,
    timestamp: u64,
) -> String {
    let result = template.replace("{basename}", basename);
    let result = result.replace("{timestamp}", &timestamp.to_string());

    match subreddit {
        Some(sub) => result.replace("{subreddit}", sub),
        None => result, // Non-split mode won't have {subreddit}
    }
}

/// Constructs the final output filename from template components
/// Returns the complete filename with extension applied if needed
fn construct_filename(
    template: &str,
    basename: &str,
    subreddit: Option<&str>,
    timestamp: u64,
    ext: &str,
    append_ext: bool,
) -> String {
    let stem = expand_placeholders(template, basename, subreddit, timestamp);

    if append_ext {
        format!("{stem}.{ext}")
    } else {
        stem
    }
}

/// This is for running a single threaded operation
/// Useful for debugging and testing, or in constrained environments
pub fn run_filter_st(args: &FilterArgs, mb: MultiProgress) -> Result<()> {
    args.input.par_iter().for_each(|input_path| {
        let file_name = input_path.to_str().expect("Failed to convert to string");

        let pb = setup_progress_bar(file_name);
        let pb = mb.add(pb);
        let (mut file_map, mut combined_writer) =
            setup_file_writers(input_path, args).expect("Failed to setup file writers");

        let reader = common::setup_reader(input_path, &pb).unwrap();
        let mut filtered_count = 0u64;

        reader.lines().map_while(Result::ok).for_each(|line| {
            if process_and_write_line(&line, args, &mut file_map, &mut combined_writer)
                .is_ok_and(|b| b)
            {
                filtered_count += 1;
                pb.set_message(format!("Filtering: {filtered_count}"));
            }
        });

        flush_writers(file_map, combined_writer).expect("Failed to flush writers");
        pb.finish_with_message(format!("Filtered: {filtered_count}"));
    });
    Ok(())
}

/// Note that there is no progress bar as the file wrapper does not know the uncompressed size
fn setup_progress_bar(filename: &str) -> ProgressBar {
    let pb = ProgressBar::no_length();
    let template =
        format!("{filename}: [{{elapsed_precise}}] {{bytes}} ({{bytes_per_sec}}) {{msg}}");
    pb.set_style(ProgressStyle::with_template(&template).expect("Failed to set progress style"));
    pb.enable_steady_tick(Duration::from_millis(10));
    pb
}

fn process_single_file(input_path: &PathBuf, args: &FilterArgs, mb: MultiProgress) -> Result<()> {
    let file_name = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let pb = setup_progress_bar(file_name);
    let pb = mb.add(pb);

    let (line_sender, line_receiver) = bounded(1_000);

    // Using rayon::join runs two closures in parallel.
    // It returns when both are finished.
    let (producer_result, consumer_result) = rayon::join(
        || {
            // Producer: Decompression
            let reader = common::setup_reader(input_path, &pb)?;
            for line in reader.lines() {
                if let Ok(l) = line {
                    if line_sender.send(l).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok::<(), Error>(())
        },
        || {
            // Consumer: JSON Processing & Writing
            let (mut file_map, mut combined_writer) = setup_file_writers(input_path, args)?;
            let mut filtered_count = 0u64;

            for line in line_receiver {
                if process_and_write_line(&line, args, &mut file_map, &mut combined_writer)
                    .is_ok_and(|b| b)
                {
                    filtered_count += 1;
                    pb.set_message(format!("Filtering: {filtered_count}"));
                }
            }

            flush_writers(file_map, combined_writer)?;
            pb.finish_with_message(format!("Filtered: {filtered_count}"));
            Ok::<(), Error>(())
        },
    );

    // Propagate the first error encountered, if any
    producer_result.and(consumer_result)
}

pub fn run_filter_mt(args: &FilterArgs, mb: MultiProgress) -> Result<()> {
    args.input.par_iter().for_each(|input_path| {
        process_single_file(input_path, args, mb.clone()).expect("Failed to process file");
    });
    Ok(())
}

/// Sets up file writers based on split mode
fn setup_file_writers<P: AsRef<std::path::Path>>(
    input_file: P,
    args: &FilterArgs,
) -> Result<(Option<FileMap>, Option<BufWriter<File>>)> {
    // Generate timestamp for placeholder expansion
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System clock is set before Unix epoch - check system time!")
        .as_secs();

    // Determine template (user-provided or default) and whether to append extension
    let (template, append_ext) = if let Some(output) = &args.output {
        (output.to_string_lossy().to_string(), false)
    } else if args.split {
        ("{basename}_{subreddit}".to_string(), true)
    } else {
        ("{basename}_filtered".to_string(), true)
    };

    // Get basename from INPUT file (not output template)
    // The {basename} placeholder always refers to the input filename
    let basename = input_file
        .as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("filtered")
        .to_string();

    // Validate parent directory exists for split mode
    if args.split && !args.name.is_empty() {
        let test_path = expand_placeholders(&template, &basename, Some(&args.name[0]), timestamp);
        if let Some(parent) = std::path::Path::new(&test_path).parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            bail!(format!(
                "Parent directory '{}' does not exist",
                parent.display()
            ),);
        }
    }

    // get the ext from outfile
    let ext = if args.output.is_some() {
        args.output
            .as_ref()
            .unwrap()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jsonl")
            .to_string()
    } else {
        String::from("zst")
    };

    if args.split {
        let map: FileMap = args
            .name
            .iter()
            .map(|name| {
                let filename = construct_filename(
                    &template,
                    &basename,
                    Some(name),
                    timestamp,
                    &ext,
                    append_ext,
                );
                let writer = common::setup_writer(filename, args.compression.level);
                (name.clone(), writer)
            })
            .collect();

        Ok((Some(map), None))
    } else {
        let filename = construct_filename(&template, &basename, None, timestamp, &ext, append_ext);
        let outfile = File::create(&filename)?;
        Ok((None, Some(BufWriter::new(outfile))))
    }
}

/// Processes a single line: extracts subreddit, filters, and writes to appropriate file
/// Returns true if the line was written, false otherwise
fn process_and_write_line(
    line: &str,
    args: &FilterArgs,
    file_map: &mut Option<FileMap>,
    combined_writer: &mut Option<BufWriter<File>>,
) -> Result<bool> {
    // Extract and filter subreddit
    let subreddit = extract_json(line);

    let subreddit_lower = subreddit.to_lowercase();

    if !args.name.contains(&subreddit_lower) {
        return Ok(false);
    }

    // Write to appropriate destination
    let written = if args.split {
        if let Some(map) = file_map {
            if let Some(writer) = map.get_mut(&subreddit_lower) {
                writeln!(writer, "{line}")?;
                true
            } else {
                false
            }
        } else {
            false
        }
    } else if let Some(writer) = combined_writer {
        writeln!(writer, "{line}")?;
        true
    } else {
        false
    };

    Ok(written)
}

/// Flushes all open writers
fn flush_writers(
    file_map: Option<FileMap>,
    combined_writer: Option<BufWriter<File>>,
) -> Result<()> {
    if let Some(map) = file_map {
        for (_, mut writer) in map {
            writer.flush()?;
        }
    } else if let Some(mut writer) = combined_writer {
        writer.flush()?;
    }
    Ok(())
}

/// Safer extraction with GJSON, as we only need the subreddit key
fn extract_json(line: &str) -> String {
    let subreddit: gjson::Value<'_> = gjson::get(line, "subreddit");

    subreddit.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nested_json() {
        // This must be tested to deal with crossposts, manual implementations of reading may fail here
        let line = r#"{"crosspost_parent_list": [{"subreddit": "foo"}],"subreddit": "bar"}"#;
        assert_eq!(extract_json(line), String::from("bar"));
    }
    #[test]
    fn test_extract() {
        // This should pass easily
        let line = r#"{"subreddit": "bar"}"#;
        assert_eq!(extract_json(line), String::from("bar"));
    }

    #[test]
    fn test_expand_basename_only() {
        let result = expand_placeholders("{basename}_filtered", "RC_2025-09", None, 100);
        assert_eq!(result, "RC_2025-09_filtered");
    }

    #[test]
    fn test_expand_with_subreddit() {
        let result = expand_placeholders("{basename}_{subreddit}", "RC_2025-09", Some("foo"), 100);
        assert_eq!(result, "RC_2025-09_foo");
    }

    #[test]
    fn test_expand_all_placeholders() {
        let result = expand_placeholders(
            "{basename}_{subreddit}_{timestamp}",
            "RC_2025-09",
            Some("foo"),
            100,
        );
        assert_eq!(result, "RC_2025-09_foo_100");
    }

    #[test]
    fn test_expand_with_directory() {
        let result = expand_placeholders(
            "outputs/{basename}_{subreddit}",
            "RC_2025-09",
            Some("foo"),
            100,
        );
        assert_eq!(result, "outputs/RC_2025-09_foo");
    }

    #[test]
    fn test_expand_timestamp_only() {
        let result = expand_placeholders("data_{timestamp}", "RC_2025-09", None, 100);
        assert_eq!(result, "data_100");
    }

    #[test]
    fn test_construct_filename_with_user_output_no_append() {
        // User provided output with extension - should not append
        let result = construct_filename(
            "processed/{basename}.zst",
            "submission-R_2025-09",
            None,
            100,
            "zst",
            false, // append_ext = false when user provides output
        );
        assert_eq!(result, "processed/submission-R_2025-09.zst");
    }

    #[test]
    fn test_construct_filename_default_template_with_append() {
        // Default template without extension - should append
        let result = construct_filename(
            "{basename}_filtered",
            "submission-R_2025-09",
            None,
            100,
            "zst",
            true, // append_ext = true for default templates
        );
        assert_eq!(result, "submission-R_2025-09_filtered.zst");
    }

    #[test]
    fn test_construct_filename_split_mode_with_append() {
        // Split mode with default template
        let result = construct_filename(
            "{basename}_{subreddit}",
            "submission-R_2025-09",
            Some("foo"),
            100,
            "zst",
            true,
        );
        assert_eq!(result, "submission-R_2025-09_foo.zst");
    }

    #[test]
    fn test_construct_filename_split_mode_user_output_no_append() {
        // Split mode with user-provided output
        let result = construct_filename(
            "outputs/{basename}_{subreddit}.jsonl",
            "submission-R_2025-09",
            Some("foo"),
            100,
            "jsonl",
            false,
        );
        assert_eq!(result, "outputs/submission-R_2025-09_foo.jsonl");
    }

    #[test]
    fn test_construct_filename_with_directory() {
        // User output with directory and extension
        let result = construct_filename(
            "processed/{basename}.zst",
            "submission-R_2025-09",
            None,
            100,
            "zst",
            false,
        );
        assert_eq!(result, "processed/submission-R_2025-09.zst");
    }
}
