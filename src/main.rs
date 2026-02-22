mod cli;
mod format;
mod inputs;
mod serialise;
mod splitter;

use std::process;

use clap::Parser;
use log::{error, info};

use crate::{
    cli::Cli,
    format::{RdfFormat, SplitterError},
    inputs::expand_inputs,
    splitter::{split_file, SplitOptions},
};

fn main() {
    let cli = Cli::parse();

    // Initialise logger
    let level = if cli.verbose { "debug" } else { "info" };
    env_logger::Builder::new()
        .filter_level(level.parse().unwrap())
        .format_target(false)
        .format_timestamp(None)
        .init();

    if let Err(e) = run(cli) {
        error!("{e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), SplitterError> {
    // Expand glob patterns / directories into concrete file paths
    let files = expand_inputs(&cli.inputs, cli.recursive)
        .map_err(SplitterError::Other)?;

    if files.is_empty() {
        return Err(SplitterError::Parse(
            "No input files found. Check your patterns or paths.".into(),
        ));
    }

    let mut total_triples = 0usize;
    let mut total_files = 0usize;
    let mut errors = 0usize;

    for path in &files {
        let fmt = match RdfFormat::from_path(path) {
            Some(f) => f,
            None => {
                log::warn!(
                    "Skipping '{}': unrecognised RDF extension",
                    path.display()
                );
                continue;
            }
        };

        // Resolve chunk size: either fixed, or derived from a desired file count.
        let chunk_size = match (cli.chunk_size, cli.file_count) {
            (_, Some(fc)) => {
                if fc == 0 {
                    log::error!("--file-count must be at least 1");
                    errors += 1;
                    continue;
                }
                log::info!("Counting records in {} …", path.display());
                match splitter::count_records(path, fmt) {
                    Ok(total) => {
                        let cs = (total + fc - 1) / fc; // ceiling division
                        log::debug!("  {} records → chunk size {}", total, cs);
                        cs.max(1)
                    }
                    Err(e) => {
                        log::error!("{}: {e}", path.display());
                        errors += 1;
                        continue;
                    }
                }
            }
            (Some(cs), _) => cs,
            (None, None) => 10_000,
        };

        let opts = SplitOptions {
            output_dir: cli.output.clone(),
            chunk_size,
            force: cli.force,
        };

        match split_file(path, fmt, &opts) {
            Ok(n) => {
                info!(
                    "{}: {} triple(s) → chunks of {}",
                    path.display(),
                    n,
                    chunk_size
                );
                total_triples += n;
                total_files += 1;
            }
            Err(e) => {
                log::error!("{}: {e}", path.display());
                errors += 1;
            }
        }
    }

    info!(
        "Done. {} file(s) processed, {} triple/quad(s) total, {} error(s).",
        total_files, total_triples, errors
    );

    if errors > 0 {
        process::exit(2);
    }

    Ok(())
}
