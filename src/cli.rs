use clap::Parser;
use std::path::PathBuf;

/// Split RDF files into smaller chunks.
///
/// Supported formats: Turtle (.ttl), N-Triples (.nt), N-Quads (.nq),
/// RDF/XML (.rdf, .owl, .xml), TriG (.trig), JSON-LD (.jsonld, .json-ld).
#[derive(Parser, Debug)]
#[command(
    name = "rdfsplitter",
    version,
    about,
    long_about = None,
    after_help = "EXAMPLES:\n  rdfsplitter data.ttl -n 1000\n  rdfsplitter data.ttl -c 4\n  rdfsplitter *.nt -n 5000 -o out/ -f\n  rdfsplitter -r src/ -c 10 -o split/"
)]
pub struct Cli {
    /// Input file(s) or glob patterns (e.g. *.ttl, data/**/*.nt)
    #[arg(required = true)]
    pub inputs: Vec<String>,

    /// Number of triples per output chunk [default: 10000, conflicts with --file-count]
    #[arg(
        short = 'n',
        long,
        value_name = "TRIPLES",
        conflicts_with = "file_count"
    )]
    pub chunk_size: Option<usize>,

    /// Split into exactly N output files (requires a counting pass; conflicts with --chunk-size)
    #[arg(
        short = 'c',
        long,
        value_name = "FILES",
        conflicts_with = "chunk_size"
    )]
    pub file_count: Option<usize>,

    /// Output directory (defaults to current directory)
    #[arg(short = 'o', long, default_value = ".", value_name = "OUTPUTDIR")]
    pub output: PathBuf,

    /// Recurse into subdirectories
    #[arg(short = 'r', long)]
    pub recursive: bool,

    /// Overwrite existing output files; create output directory if missing
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Verbose log output
    #[arg(short = 'v', long)]
    pub verbose: bool,
}
