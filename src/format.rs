use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdfFormat {
    Turtle,
    NTriples,
    NQuads,
    TriG,
    RdfXml,
    JsonLd,
}

impl RdfFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "ttl" => Some(Self::Turtle),
            "nt" => Some(Self::NTriples),
            "nq" | "nquads" => Some(Self::NQuads),
            "trig" => Some(Self::TriG),
            "rdf" | "owl" | "xml" => Some(Self::RdfXml),
            "jsonld" | "json-ld" | "json" => Some(Self::JsonLd),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Turtle => "ttl",
            Self::NTriples => "nt",
            Self::NQuads => "nq",
            Self::TriG => "trig",
            Self::RdfXml => "rdf",
            Self::JsonLd => "jsonld",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Turtle => "Turtle",
            Self::NTriples => "N-Triples",
            Self::NQuads => "N-Quads",
            Self::TriG => "TriG",
            Self::RdfXml => "RDF/XML",
            Self::JsonLd => "JSON-LD",
        }
    }
}

/// Callback error type for rio `parse_all` closures.
/// rio_api requires `From<ParserError>` on the callback's error type.
#[derive(Debug)]
pub struct CallbackError(pub String);

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CallbackError {}

impl From<rio_turtle::TurtleError> for CallbackError {
    fn from(e: rio_turtle::TurtleError) -> Self {
        CallbackError(e.to_string())
    }
}

impl From<rio_xml::RdfXmlError> for CallbackError {
    fn from(e: rio_xml::RdfXmlError) -> Self {
        CallbackError(e.to_string())
    }
}

#[derive(Debug, Error)]
pub enum SplitterError {
    #[allow(dead_code)]
    #[error("Unsupported format for '{0}'; supported: .ttl .nt .nq .trig .rdf .owl .xml .jsonld")]
    UnsupportedFormat(String),

    #[error("Output directory '{0}' does not exist (use --force to create it)")]
    OutputDirMissing(String),

    #[error("Output file '{0}' already exists (use --force to overwrite)")]
    OutputExists(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("RDF parse error: {0}")]
    Parse(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detect_format_from_all_supported_extensions() {
        let cases: &[(&str, RdfFormat)] = &[
            ("file.ttl",    RdfFormat::Turtle),
            ("file.nt",     RdfFormat::NTriples),
            ("file.nq",     RdfFormat::NQuads),
            ("file.nquads", RdfFormat::NQuads),
            ("file.trig",   RdfFormat::TriG),
            ("file.rdf",    RdfFormat::RdfXml),
            ("file.owl",    RdfFormat::RdfXml),
            ("file.xml",    RdfFormat::RdfXml),
            ("file.jsonld", RdfFormat::JsonLd),
            ("file.json",   RdfFormat::JsonLd),
        ];
        for (filename, expected) in cases {
            assert_eq!(
                RdfFormat::from_path(Path::new(filename)),
                Some(*expected),
                "failed for {filename}"
            );
        }
    }

    #[test]
    fn detect_format_is_case_insensitive() {
        assert_eq!(RdfFormat::from_path(Path::new("A.TTL")),    Some(RdfFormat::Turtle));
        assert_eq!(RdfFormat::from_path(Path::new("A.NT")),     Some(RdfFormat::NTriples));
        assert_eq!(RdfFormat::from_path(Path::new("A.JSONLD")), Some(RdfFormat::JsonLd));
        assert_eq!(RdfFormat::from_path(Path::new("A.NQ")),     Some(RdfFormat::NQuads));
        assert_eq!(RdfFormat::from_path(Path::new("A.RDF")),    Some(RdfFormat::RdfXml));
    }

    #[test]
    fn unknown_extension_returns_none() {
        assert_eq!(RdfFormat::from_path(Path::new("file.txt")),  None);
        assert_eq!(RdfFormat::from_path(Path::new("file.csv")),  None);
        assert_eq!(RdfFormat::from_path(Path::new("no_extension")), None);
    }

    #[test]
    fn extension_roundtrips_through_from_path() {
        let formats = [
            RdfFormat::Turtle,
            RdfFormat::NTriples,
            RdfFormat::NQuads,
            RdfFormat::TriG,
            RdfFormat::RdfXml,
            RdfFormat::JsonLd,
        ];
        for fmt in formats {
            let path = std::path::PathBuf::from(format!("test.{}", fmt.extension()));
            assert_eq!(
                RdfFormat::from_path(&path),
                Some(fmt),
                "{} extension '{}' not round-tripped",
                fmt.label(),
                fmt.extension()
            );
        }
    }

    #[test]
    fn label_is_non_empty_for_all_variants() {
        for fmt in [
            RdfFormat::Turtle,
            RdfFormat::NTriples,
            RdfFormat::NQuads,
            RdfFormat::TriG,
            RdfFormat::RdfXml,
            RdfFormat::JsonLd,
        ] {
            assert!(!fmt.label().is_empty());
        }
    }
}
