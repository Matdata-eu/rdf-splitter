use std::path::PathBuf;

use anyhow::Context;
use glob::glob;
use log::warn;

use crate::format::RdfFormat;

/// Expand a list of input patterns (may contain globs) into concrete file
/// paths.  If `recursive` is true and a pattern is a bare directory, walk it
/// for known RDF extensions.
pub fn expand_inputs(patterns: &[String], recursive: bool) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = Vec::new();

    for pattern in patterns {
        let p = std::path::Path::new(pattern);

        // bare existing directory → walk
        if p.is_dir() {
            let dir_files = walk_dir(p, recursive);
            if dir_files.is_empty() {
                warn!("No RDF files found in directory '{}'", pattern);
            }
            paths.extend(dir_files);
            continue;
        }

        // treat as glob
        let matches: Vec<_> = glob(pattern)
            .with_context(|| format!("Invalid glob pattern: '{pattern}'"))?
            .filter_map(|r| match r {
                Ok(p) => Some(p),
                Err(e) => {
                    warn!("Glob error: {e}");
                    None
                }
            })
            .filter(|p| p.is_file())
            .collect();

        if matches.is_empty() {
            warn!("No files matched pattern '{pattern}'");
        }

        // If recursive flag and we matched directories, walk them
        for m in matches {
            if m.is_dir() {
                paths.extend(walk_dir(&m, recursive));
            } else {
                paths.push(m);
            }
        }
    }

    // de-duplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));

    Ok(paths)
}

fn walk_dir(dir: &std::path::Path, recursive: bool) -> Vec<PathBuf> {
    let mut results = Vec::new();

    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) => {
            warn!("Cannot read directory '{}': {e}", dir.display());
            return results;
        }
    };

    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() && recursive {
            results.extend(walk_dir(&path, recursive));
        } else if path.is_file() && RdfFormat::from_path(&path).is_some() {
            results.push(path);
        }
    }

    results
}
