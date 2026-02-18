use ahash::{AHashMap, AHashSet};
use anyhow::{Context, Result, bail};
use gjson;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use regex::Regex;
use std::io::{BufRead, Write};
use std::time::Duration;

use crate::cli::FilterArgs;
use crate::common;

type FileMap = AHashMap<String, Box<dyn Write>>;

/// Represents the two modes of filtered output writing.
enum FilterWriter {
    /// Split mode: each subreddit gets its own file. The HashMap keys act as the filter.
    Split(FileMap),
    /// Combined mode: all matching lines go to one file, filtered by the HashSet.
    Combined {
        set: AHashSet<String>,
        writer: Box<dyn Write>,
    },
}

pub fn validate_args(args: &FilterArgs) -> Result<()> {
    let valid_placeholders = &["basename", "subreddit", "timestamp"];

    if let Some(output) = &args.output {
        let output_str = output.to_string_lossy();

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

/// Processes input files in parallel using rayon
/// Each input file is processed independently with its own progress bar and file writers
pub fn run_filter(args: &FilterArgs, mb: MultiProgress) -> Result<()> {
    args.input.par_iter().for_each(|input_path| {
        let file_name = match input_path.to_str() {
            Some(s) => s,
            None => {
                eprintln!("Error: Invalid UTF-8 in path: {}", input_path.display());
                return;
            }
        };

        let pb = setup_progress_bar(file_name);
        let pb = mb.add(pb);
        let mut filter_writer = match setup_filter_writer(input_path, args) {
            Ok(w) => w,
            Err(e) => {
                eprintln!(
                    "Error setting up writers for {}: {}",
                    input_path.display(),
                    e
                );
                return;
            }
        };

        let reader = match common::setup_reader(input_path, &pb) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error opening reader for {}: {}", input_path.display(), e);
                return;
            }
        };
        let mut filtered_count = 0u64;

        reader.lines().map_while(Result::ok).for_each(|line| {
            if process_and_write_line(&line, &mut filter_writer).is_ok_and(|b| b) {
                filtered_count += 1;
                pb.set_message(format!("Filtering: {filtered_count}"));
            }
        });

        flush_writer(filter_writer).expect("Failed to flush writers");
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
/// Sets up the appropriate FilterWriter based on split mode
fn setup_filter_writer<P: AsRef<std::path::Path>>(
    input_file: P,
    args: &FilterArgs,
) -> Result<FilterWriter> {
    // Generate timestamp for placeholder expansion
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        // would this ever happen?
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

    // Get basename from the input
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
    let ext = if let Some(output) = &args.output {
        output
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
                let writer = common::setup_writer(filename, &args.compression)
                    .expect("Unable to setup writer");
                (name.clone(), writer)
            })
            .collect();

        Ok(FilterWriter::Split(map))
    } else {
        let filename = construct_filename(&template, &basename, None, timestamp, &ext, append_ext);
        let writer = common::setup_writer(filename, &args.compression)?;
        let set = args.name.iter().cloned().collect::<AHashSet<String>>();
        Ok(FilterWriter::Combined { set, writer })
    }
}

/// Processes a single line: extracts subreddit, filters, and writes to appropriate file
/// Returns true if the line was written, false otherwise
fn process_and_write_line(line: &str, filter_writer: &mut FilterWriter) -> Result<bool> {
    let subreddit_key = extract_json(line).str().to_lowercase();

    match filter_writer {
        FilterWriter::Split(map) => {
            if let Some(writer) = map.get_mut(&subreddit_key) {
                writeln!(writer, "{line}")?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        FilterWriter::Combined { set, writer } => {
            if !set.contains(&subreddit_key) {
                return Ok(false);
            }
            writeln!(writer, "{line}")?;
            Ok(true)
        }
    }
}

/// Flushes all open writers
fn flush_writer(filter_writer: FilterWriter) -> Result<()> {
    match filter_writer {
        FilterWriter::Split(map) => {
            for (_, mut writer) in map {
                writer.flush()?;
            }
        }
        FilterWriter::Combined { mut writer, .. } => {
            writer.flush()?;
        }
    }
    Ok(())
}

/// Safer extraction with GJSON, as we only need the subreddit key
// Returns gjson::Value which references data in the original line buffer.
fn extract_json(line: &str) -> gjson::Value<'_> {
    gjson::get(line, "subreddit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nested_json() {
        // This must be tested to deal with crossposts, manual implementations of reading may fail here
        let line = r#"{"crosspost_parent_list": [{"subreddit": "foo"}],"subreddit": "bar"}"#;
        assert_eq!(extract_json(line).str(), "bar");
    }
    #[test]
    fn test_extract() {
        // This should pass easily
        let line = r#"{"subreddit": "bar"}"#;
        assert_eq!(extract_json(line).str(), "bar");
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
