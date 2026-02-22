use std::{
    fs,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use log::{debug, info};
use oxiri::Iri;
use rio_api::parser::{QuadsParser, TriplesParser};
use rio_turtle::{NQuadsParser, NTriplesParser, TriGParser, TurtleParser};
use rio_xml::RdfXmlParser;

use crate::{
    format::{CallbackError, RdfFormat, SplitterError},
    serialise::{
        write_jsonld, write_nquads, write_ntriples, write_rdfxml, write_trig, write_turtle,
        OwnedQuad, OwnedTriple,
    },
};

/// Print an in-place progress counter to stderr every [`PROGRESS_INTERVAL`] records.
const PROGRESS_INTERVAL: usize = 100_000;

fn show_progress(n: usize) {
    use std::io::Write;
    eprint!("\r  {:>12} records...", n);
    let _ = std::io::stderr().flush();
}

/// Erase the progress line so subsequent log output starts on a clean line.
fn clear_progress() {
    eprint!("\r{:40}\r", "");
}

pub struct SplitOptions {
    pub output_dir: PathBuf,
    pub chunk_size: usize,
    pub force: bool,
}

/// Count the total number of triples/quads in a file without storing them.
/// Used by `--file-count` to compute the required chunk size.
pub fn count_records(input: &Path, fmt: RdfFormat) -> Result<usize, SplitterError> {
    let file = fs::File::open(input)?;
    let reader = BufReader::new(file);
    let base_str = file_base_iri(input);
    let mut n = 0usize;

    match fmt {
        RdfFormat::NTriples => {
            let mut p = NTriplesParser::new(reader);
            p.parse_all(&mut |_: rio_api::model::Triple<'_>| -> Result<(), CallbackError> {
                n += 1;
                if n % PROGRESS_INTERVAL == 0 { show_progress(n); }
                Ok(())
            })
            .map_err(|e| SplitterError::Parse(e.to_string()))?;
        }
        RdfFormat::Turtle => {
            let base = Iri::parse(base_str).map_err(|e| SplitterError::Parse(e.to_string()))?;
            let mut p = TurtleParser::new(reader, Some(base));
            p.parse_all(&mut |_: rio_api::model::Triple<'_>| -> Result<(), CallbackError> {
                n += 1;
                if n % PROGRESS_INTERVAL == 0 { show_progress(n); }
                Ok(())
            })
            .map_err(|e| SplitterError::Parse(e.to_string()))?;
        }
        RdfFormat::RdfXml => {
            let base = Iri::parse(base_str).map_err(|e| SplitterError::Parse(e.to_string()))?;
            let mut p = RdfXmlParser::new(reader, Some(base));
            p.parse_all(&mut |_: rio_api::model::Triple<'_>| -> Result<(), CallbackError> {
                n += 1;
                if n % PROGRESS_INTERVAL == 0 { show_progress(n); }
                Ok(())
            })
            .map_err(|e| SplitterError::Parse(e.to_string()))?;
        }
        RdfFormat::NQuads => {
            let mut p = NQuadsParser::new(reader);
            p.parse_all(&mut |_: rio_api::model::Quad<'_>| -> Result<(), CallbackError> {
                n += 1;
                if n % PROGRESS_INTERVAL == 0 { show_progress(n); }
                Ok(())
            })
            .map_err(|e| SplitterError::Parse(e.to_string()))?;
        }
        RdfFormat::TriG => {
            let base = Iri::parse(base_str).map_err(|e| SplitterError::Parse(e.to_string()))?;
            let mut p = TriGParser::new(reader, Some(base));
            p.parse_all(&mut |_: rio_api::model::Quad<'_>| -> Result<(), CallbackError> {
                n += 1;
                if n % PROGRESS_INTERVAL == 0 { show_progress(n); }
                Ok(())
            })
            .map_err(|e| SplitterError::Parse(e.to_string()))?;
        }
        RdfFormat::JsonLd => {
            let raw = fs::read_to_string(input)?;
            let nt = jsonld_to_ntriples(&raw)?;
            n = nt.lines().filter(|l| !l.trim().is_empty()).count();
        }
    }
    clear_progress();

    Ok(n)
}

/// Split a single file into chunks.  Returns the number of triples/quads processed.
pub fn split_file(
    input: &Path,
    fmt: RdfFormat,
    opts: &SplitOptions,
) -> Result<usize, SplitterError> {
    prepare_output_dir(&opts.output_dir, opts.force)?;
    info!("Splitting {} [{}]", input.display(), fmt.label());

    match fmt {
        RdfFormat::NTriples | RdfFormat::Turtle | RdfFormat::RdfXml => {
            split_triples(input, fmt, opts)
        }
        RdfFormat::NQuads | RdfFormat::TriG => split_quads(input, fmt, opts),
        RdfFormat::JsonLd => split_jsonld_file(input, opts),
    }
}

// ─── triple-based formats ───────────────────────────────────────────────────

fn split_triples(
    input: &Path,
    fmt: RdfFormat,
    opts: &SplitOptions,
) -> Result<usize, SplitterError> {
    let base_str = file_base_iri(input);

    let mut triples: Vec<OwnedTriple> = Vec::with_capacity(opts.chunk_size);
    let mut chunk = 0usize;
    let mut total = 0usize;
    let mut flush_err: Option<SplitterError> = None;

    {
        let file = fs::File::open(input)?;
        let reader = BufReader::new(file);

        let flush = |triples: &mut Vec<OwnedTriple>,
                     chunk: &mut usize,
                     total: &mut usize,
                     flush_err: &mut Option<SplitterError>| {
            if triples.is_empty() {
                return;
            }
            match write_triple_chunk(input, fmt, triples, *chunk, opts) {
                Ok(()) => {
                    *chunk += 1;
                    *total += triples.len();
                    triples.clear();
                }
                Err(e) => {
                    *flush_err = Some(e);
                }
            }
        };

        let mut parsed = 0usize;
        let mut on_triple = |t: rio_api::model::Triple<'_>| -> Result<(), CallbackError> {
            triples.push(OwnedTriple::from_rio(&t));
            parsed += 1;
            if parsed % PROGRESS_INTERVAL == 0 { show_progress(parsed); }
            if triples.len() >= opts.chunk_size {
                flush(&mut triples, &mut chunk, &mut total, &mut flush_err);
            }
            Ok(())
        };

        match fmt {
            RdfFormat::NTriples => {
                let mut parser = NTriplesParser::new(reader);
                parser
                    .parse_all(&mut on_triple)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
            }
            RdfFormat::Turtle => {
                let base = Iri::parse(base_str)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
                let mut parser = TurtleParser::new(reader, Some(base));
                parser
                    .parse_all(&mut on_triple)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
            }
            RdfFormat::RdfXml => {
                let base = Iri::parse(base_str)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
                let mut parser = RdfXmlParser::new(reader, Some(base));
                parser
                    .parse_all(&mut on_triple)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
            }
            _ => unreachable!(),
        }
    }

    clear_progress();
    if let Some(e) = flush_err {
        return Err(e);
    }

    // flush remainder
    if !triples.is_empty() {
        write_triple_chunk(input, fmt, &triples, chunk, opts)?;
        total += triples.len();
    }

    Ok(total)
}

fn write_triple_chunk(
    input: &Path,
    fmt: RdfFormat,
    triples: &[OwnedTriple],
    chunk: usize,
    opts: &SplitOptions,
) -> Result<(), SplitterError> {
    let out_path = chunk_path(input, fmt, chunk, opts);
    check_overwrite(&out_path, opts.force)?;
    debug!("  writing chunk {} → {}", chunk, out_path.display());
    let file = fs::File::create(&out_path)?;
    let mut w = BufWriter::new(file);
    match fmt {
        RdfFormat::NTriples => write_ntriples(&mut w, triples)?,
        RdfFormat::Turtle => write_turtle(&mut w, triples)?,
        RdfFormat::RdfXml => write_rdfxml(&mut w, triples)?,
        _ => unreachable!(),
    }
    Ok(())
}

// ─── quad-based formats ─────────────────────────────────────────────────────

fn split_quads(
    input: &Path,
    fmt: RdfFormat,
    opts: &SplitOptions,
) -> Result<usize, SplitterError> {
    let base_str = file_base_iri(input);

    let mut quads: Vec<OwnedQuad> = Vec::with_capacity(opts.chunk_size);
    let mut chunk = 0usize;
    let mut total = 0usize;
    let mut flush_err: Option<SplitterError> = None;

    {
        let file = fs::File::open(input)?;
        let reader = BufReader::new(file);

        let flush = |quads: &mut Vec<OwnedQuad>,
                     chunk: &mut usize,
                     total: &mut usize,
                     flush_err: &mut Option<SplitterError>| {
            if quads.is_empty() {
                return;
            }
            match write_quad_chunk(input, fmt, quads, *chunk, opts) {
                Ok(()) => {
                    *chunk += 1;
                    *total += quads.len();
                    quads.clear();
                }
                Err(e) => {
                    *flush_err = Some(e);
                }
            }
        };

        let mut parsed = 0usize;
        let mut on_quad = |q: rio_api::model::Quad<'_>| -> Result<(), CallbackError> {
            quads.push(OwnedQuad::from_rio(&q));
            parsed += 1;
            if parsed % PROGRESS_INTERVAL == 0 { show_progress(parsed); }
            if quads.len() >= opts.chunk_size {
                flush(&mut quads, &mut chunk, &mut total, &mut flush_err);
            }
            Ok(())
        };

        match fmt {
            RdfFormat::NQuads => {
                let mut parser = NQuadsParser::new(reader);
                parser
                    .parse_all(&mut on_quad)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
            }
            RdfFormat::TriG => {
                let base = Iri::parse(base_str)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
                let mut parser = TriGParser::new(reader, Some(base));
                parser
                    .parse_all(&mut on_quad)
                    .map_err(|e| SplitterError::Parse(e.to_string()))?;
            }
            _ => unreachable!(),
        }
    }

    clear_progress();
    if let Some(e) = flush_err {
        return Err(e);
    }

    if !quads.is_empty() {
        write_quad_chunk(input, fmt, &quads, chunk, opts)?;
        total += quads.len();
    }

    Ok(total)
}

fn write_quad_chunk(
    input: &Path,
    fmt: RdfFormat,
    quads: &[OwnedQuad],
    chunk: usize,
    opts: &SplitOptions,
) -> Result<(), SplitterError> {
    let out_path = chunk_path(input, fmt, chunk, opts);
    check_overwrite(&out_path, opts.force)?;
    debug!("  writing chunk {} → {}", chunk, out_path.display());
    let file = fs::File::create(&out_path)?;
    let mut w = BufWriter::new(file);
    match fmt {
        RdfFormat::NQuads => write_nquads(&mut w, quads)?,
        RdfFormat::TriG => write_trig(&mut w, quads)?,
        _ => unreachable!(),
    }
    Ok(())
}

// ─── JSON-LD ─────────────────────────────────────────────────────────────────

fn split_jsonld_file(input: &Path, opts: &SplitOptions) -> Result<usize, SplitterError> {
    info!("  loading and converting JSON-LD...");
    let raw = fs::read_to_string(input)?;
    let nt_string = jsonld_to_ntriples(&raw)?;

    let cursor = std::io::Cursor::new(nt_string.as_bytes());
    let reader = BufReader::new(cursor);

    let mut triples: Vec<OwnedTriple> = Vec::with_capacity(opts.chunk_size);
    let mut chunk = 0usize;
    let mut total = 0usize;
    let mut flush_err: Option<SplitterError> = None;

    let flush = |triples: &mut Vec<OwnedTriple>,
                 chunk: &mut usize,
                 total: &mut usize,
                 flush_err: &mut Option<SplitterError>| {
        if triples.is_empty() {
            return;
        }
        let out_path = chunk_path(input, RdfFormat::JsonLd, *chunk, opts);
        let result = (|| -> Result<(), SplitterError> {
            check_overwrite(&out_path, opts.force)?;
            debug!("  writing chunk {} → {}", chunk, out_path.display());
            let file = fs::File::create(&out_path)?;
            let mut w = BufWriter::new(file);
            write_jsonld(&mut w, triples)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                *chunk += 1;
                *total += triples.len();
                triples.clear();
            }
            Err(e) => *flush_err = Some(e),
        }
    };

    let mut parsed = 0usize;
    let mut parser = NTriplesParser::new(reader);
    parser
        .parse_all(&mut |t: rio_api::model::Triple<'_>| -> Result<(), CallbackError> {
            triples.push(OwnedTriple::from_rio(&t));
            parsed += 1;
            if parsed % PROGRESS_INTERVAL == 0 { show_progress(parsed); }
            if triples.len() >= opts.chunk_size {
                flush(&mut triples, &mut chunk, &mut total, &mut flush_err);
            }
            Ok(())
        })
        .map_err(|e| SplitterError::Parse(e.to_string()))?;
    clear_progress();

    if let Some(e) = flush_err {
        return Err(e);
    }

    if !triples.is_empty() {
        let out_path = chunk_path(input, RdfFormat::JsonLd, chunk, opts);
        check_overwrite(&out_path, opts.force)?;
        debug!("  writing chunk {} → {}", chunk, out_path.display());
        let file = fs::File::create(&out_path)?;
        let mut w = BufWriter::new(file);
        write_jsonld(&mut w, &triples)?;
        total += triples.len();
    }

    Ok(total)
}

/// Convert JSON-LD string to N-Triples via serde_json structural walk.
fn jsonld_to_ntriples(raw: &str) -> Result<String, SplitterError> {
    use serde_json::Value;
    let v: Value =
        serde_json::from_str(raw).map_err(|e| SplitterError::Parse(e.to_string()))?;

    let mut out = String::new();
    match &v {
        Value::Array(arr) => {
            for node in arr {
                extract_node(node, None, &mut out);
            }
        }
        Value::Object(_) => {
            extract_node(&v, None, &mut out);
        }
        _ => {}
    }
    Ok(out)
}

fn expand_iri(s: &str) -> String {
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("urn:") {
        format!("<{s}>")
    } else if s.starts_with("_:") {
        s.to_owned()
    } else {
        format!("<{s}>")
    }
}

fn extract_node(node: &serde_json::Value, graph: Option<&str>, out: &mut String) {
    use serde_json::Value;
    let obj = match node.as_object() {
        Some(o) => o,
        None => return,
    };

    if let Some(Value::Array(graph_nodes)) = obj.get("@graph") {
        let g = obj
            .get("@id")
            .and_then(|v| v.as_str())
            .map(|s| expand_iri(s));
        for n in graph_nodes {
            extract_node(n, g.as_deref(), out);
        }
        return;
    }

    let subject = match obj.get("@id").and_then(|v| v.as_str()) {
        Some(id) => expand_iri(id),
        None => return,
    };

    for (key, values) in obj {
        if key == "@id" || key == "@context" {
            continue;
        }
        let predicate = if key == "@type" {
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_owned()
        } else {
            expand_iri(key)
        };

        let vals: Vec<&Value> = match values {
            Value::Array(a) => a.iter().collect(),
            other => vec![other],
        };

        for val in vals {
            if let Some(o) = jsonld_value_to_nt_object(key, val) {
                if let Some(g) = graph {
                    out.push_str(&format!("{subject} {predicate} {o} {g} .\n"));
                } else {
                    out.push_str(&format!("{subject} {predicate} {o} .\n"));
                }
            }
        }
    }
}

fn jsonld_value_to_nt_object(key: &str, val: &serde_json::Value) -> Option<String> {
    use serde_json::Value;
    match val {
        Value::Object(m) => {
            if let Some(iri) = m.get("@id").and_then(|v| v.as_str()) {
                return Some(expand_iri(iri));
            }
            let value = m.get("@value")?.as_str()?;
            if let Some(lang) = m.get("@language").and_then(|v| v.as_str()) {
                return Some(format!(r#""{}"@{}"#, nt_escape(value), lang));
            }
            if let Some(dt) = m.get("@type").and_then(|v| v.as_str()) {
                return Some(format!(
                    r#""{}"^^{}"#,
                    nt_escape(value),
                    expand_iri(dt)
                ));
            }
            Some(format!(r#""{}""#, nt_escape(value)))
        }
        Value::String(s) => {
            if key == "@type" {
                Some(expand_iri(s))
            } else {
                Some(format!(r#""{}""#, nt_escape(s)))
            }
        }
        Value::Bool(b) => Some(format!(
            r#""{}"^^<http://www.w3.org/2001/XMLSchema#boolean>"#,
            b
        )),
        Value::Number(n) => Some(format!(
            r#""{}"^^<http://www.w3.org/2001/XMLSchema#decimal>"#,
            n
        )),
        _ => None,
    }
}

fn nt_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ─── path helpers ────────────────────────────────────────────────────────────

fn file_base_iri(path: &Path) -> String {
    // Produce a valid file:/// IRI usable as RDF base
    let abs = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    let s = abs.display().to_string().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

fn chunk_path(input: &Path, fmt: RdfFormat, chunk: usize, opts: &SplitOptions) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let name = format!("{}_{:04}.{}", stem, chunk, fmt.extension());
    opts.output_dir.join(name)
}

fn check_overwrite(path: &Path, force: bool) -> Result<(), SplitterError> {
    if path.exists() && !force {
        return Err(SplitterError::OutputExists(path.display().to_string()));
    }
    Ok(())
}

fn prepare_output_dir(dir: &Path, force: bool) -> Result<(), SplitterError> {
    if dir.exists() {
        return Ok(());
    }
    if !force {
        return Err(SplitterError::OutputDirMissing(dir.display().to_string()));
    }
    fs::create_dir_all(dir)?;
    Ok(())
}
