# rdfsplitter

Split large RDF files into smaller chunks.

**Formats:** Turtle (`.ttl`), N-Triples (`.nt`), N-Quads (`.nq`), TriG (`.trig`), RDF/XML (`.rdf` `.owl` `.xml`), JSON-LD (`.jsonld`)

## Install

```sh
cargo install rdfsplitter
```

**Docker:**
```sh
docker pull ghcr.io/matdata-eu/rdfsplitter:latest
```

## Usage

```
rdfsplitter [OPTIONS] <INPUT>...

Arguments:
  <INPUT>...  Files or glob patterns (e.g. *.ttl, data/**/*.nt)

Options:
  -n, --chunk-size <TRIPLES>  Triples per output chunk [default: 10000, conflicts with -c]
  -c, --file-count <FILES>    Split into exactly N output files (counts first; conflicts with -n)
  -o, --output <OUTPUTDIR>    Output directory [default: .]
  -r, --recursive             Recurse into subdirectories
  -f, --force                 Overwrite existing files; create output dir if missing
  -v, --verbose               Verbose log output
  -h, --help                  Print help
  -V, --version               Print version
```

## Examples

```sh
# Split a Turtle file into 1 000-triple chunks
rdfsplitter data.ttl -n 1000

# Split a file into exactly 4 output files
rdfsplitter data.ttl -c 4

# Split all N-Triples files in a directory tree into output/
rdfsplitter -r data/ -n 5000 -o output/ -f

# Docker
docker run --rm -v "$PWD:/data" ghcr.io/matdata-eu/rdfsplitter *.ttl -n 1000 -f
```

Output files are named `<stem>_<NNNN>.<ext>` (e.g. `data_0000.ttl`, `data_0001.ttl`, …).
