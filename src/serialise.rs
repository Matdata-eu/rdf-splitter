use std::io::Write;

use rio_api::model::{Quad, Triple};

/// A lightweight serialisable triple (owned strings).
#[derive(Debug, Clone)]
pub struct OwnedTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// A lightweight serialisable quad (triple + optional graph name).
#[derive(Debug, Clone)]
pub struct OwnedQuad {
    pub triple: OwnedTriple,
    pub graph_name: Option<String>,
}

impl OwnedTriple {
    pub fn from_rio(t: &Triple<'_>) -> Self {
        Self {
            subject: t.subject.to_string(),
            predicate: t.predicate.to_string(),
            object: t.object.to_string(),
        }
    }
}

impl OwnedQuad {
    pub fn from_rio(q: &Quad<'_>) -> Self {
        Self {
            triple: OwnedTriple {
                subject: q.subject.to_string(),
                predicate: q.predicate.to_string(),
                object: q.object.to_string(),
            },
            graph_name: q.graph_name.map(|g| g.to_string()),
        }
    }
}

// ─── Writers ───────────────────────────────────────────────────────────────

pub fn write_ntriples<W: Write>(
    w: &mut W,
    triples: &[OwnedTriple],
) -> std::io::Result<()> {
    for t in triples {
        writeln!(w, "{} {} {} .", t.subject, t.predicate, t.object)?;
    }
    Ok(())
}

pub fn write_nquads<W: Write>(
    w: &mut W,
    quads: &[OwnedQuad],
) -> std::io::Result<()> {
    for q in quads {
        if let Some(g) = &q.graph_name {
            writeln!(
                w,
                "{} {} {} {} .",
                q.triple.subject, q.triple.predicate, q.triple.object, g
            )?;
        } else {
            writeln!(
                w,
                "{} {} {} .",
                q.triple.subject, q.triple.predicate, q.triple.object
            )?;
        }
    }
    Ok(())
}

/// Write a minimal valid Turtle chunk.
/// We serialise as N-Triples inside a .ttl file since N-Triples is a
/// valid subset of Turtle, keeping the output parse-able with any Turtle
/// parser while avoiding the complexity of prefix round-tripping.
pub fn write_turtle<W: Write>(
    w: &mut W,
    triples: &[OwnedTriple],
) -> std::io::Result<()> {
    // N-Triples syntax is valid Turtle
    write_ntriples(w, triples)
}

/// Write a minimal valid TriG chunk (N-Quads is valid TriG).
pub fn write_trig<W: Write>(
    w: &mut W,
    quads: &[OwnedQuad],
) -> std::io::Result<()> {
    write_nquads(w, quads)
}

/// Write RDF/XML for a chunk of triples.
pub fn write_rdfxml<W: Write>(
    w: &mut W,
    triples: &[OwnedTriple],
) -> std::io::Result<()> {
    writeln!(
        w,
        r#"<?xml version="1.0" encoding="utf-8"?>"#
    )?;
    writeln!(
        w,
        r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#
    )?;
    for t in triples {
        // subject
        let subj = strip_angles(&t.subject);
        let pred = strip_angles(&t.predicate);
        writeln!(
            w,
            r#"  <rdf:Description rdf:about="{}">"#,
            xml_escape(subj)
        )?;
        if let Some(obj_iri) = try_strip_angles(&t.object) {
            writeln!(
                w,
                r#"    <{} rdf:resource="{}"/>"#,
                pred,
                xml_escape(obj_iri)
            )?;
        } else if let Some((lit, lang)) = try_lang_literal(&t.object) {
            writeln!(
                w,
                r#"    <{} xml:lang="{}">{}</{}>"#,
                pred,
                lang,
                xml_escape(lit),
                pred
            )?;
        } else if let Some((lit, dt)) = try_typed_literal(&t.object) {
            writeln!(
                w,
                r#"    <{} rdf:datatype="{}">{}</{}>"#,
                pred,
                xml_escape(dt),
                xml_escape(lit),
                pred
            )?;
        } else {
            // plain literal
            let lit = plain_literal(&t.object);
            writeln!(w, r#"    <{}>{}</{}>"#, pred, xml_escape(lit), pred)?;
        }
        writeln!(w, r#"  </rdf:Description>"#)?;
    }
    writeln!(w, r#"</rdf:RDF>"#)?;
    Ok(())
}

/// Write JSON-LD for a chunk of triples (expanded form, no context).
pub fn write_jsonld<W: Write>(
    w: &mut W,
    triples: &[OwnedTriple],
) -> std::io::Result<()> {
    // Group by subject for a cleaner output
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, Vec<(&OwnedTriple, &str)>> = BTreeMap::new();
    for t in triples {
        map.entry(t.subject.clone())
            .or_default()
            .push((t, &t.predicate));
    }

    writeln!(w, "[")?;
    let subjects: Vec<_> = map.keys().cloned().collect();
    for (si, subj) in subjects.iter().enumerate() {
        let entries = &map[subj];
        let subj_iri = try_strip_angles(subj).unwrap_or(subj.as_str());
        writeln!(w, "  {{")?;
        writeln!(w, r#"    "@id": "{}","#, json_escape(subj_iri))?;
        // group by predicate
        let mut by_pred: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (t, _) in entries {
            by_pred
                .entry(t.predicate.clone())
                .or_default()
                .push(object_to_jsonld_value(&t.object));
        }
        let preds: Vec<_> = by_pred.keys().cloned().collect();
        for (pi, pred) in preds.iter().enumerate() {
            let pred_str = try_strip_angles(pred).unwrap_or(pred.as_str());
            let values = &by_pred[pred];
            let trailing = if pi + 1 < preds.len() { "," } else { "" };
            if values.len() == 1 {
                writeln!(
                    w,
                    r#"    "{}": [{}]{}"#,
                    json_escape(pred_str),
                    values[0],
                    trailing
                )?;
            } else {
                writeln!(w, r#"    "{}": ["#, json_escape(pred_str))?;
                for (vi, v) in values.iter().enumerate() {
                    let comma = if vi + 1 < values.len() { "," } else { "" };
                    writeln!(w, "      {}{}", v, comma)?;
                }
                writeln!(w, r#"    ]{}"#, trailing)?;
            }
        }
        let comma = if si + 1 < subjects.len() { "," } else { "" };
        writeln!(w, "  }}{}", comma)?;
    }
    writeln!(w, "]")?;
    Ok(())
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn strip_angles(s: &str) -> &str {
    try_strip_angles(s).unwrap_or(s)
}

fn try_strip_angles(s: &str) -> Option<&str> {
    if s.starts_with('<') && s.ends_with('>') {
        Some(&s[1..s.len() - 1])
    } else {
        None
    }
}

/// `"foo"@en` → Some(("foo", "en"))
fn try_lang_literal(s: &str) -> Option<(&str, &str)> {
    if let Some(pos) = s.rfind("\"@") {
        let lang = &s[pos + 2..];
        let lit = s.trim_start_matches('"');
        let lit = &lit[..lit.rfind('"').unwrap_or(lit.len())];
        Some((lit, lang))
    } else {
        None
    }
}

/// `"foo"^^<dt>` → Some(("foo", "dt-iri"))
fn try_typed_literal(s: &str) -> Option<(&str, &str)> {
    if let Some(pos) = s.find("\"^^<") {
        let lit = s.trim_start_matches('"');
        let lit = &lit[..lit.find('"').unwrap_or(lit.len())];
        let dt = &s[pos + 4..s.len() - 1];
        Some((lit, dt))
    } else {
        None
    }
}

fn plain_literal(s: &str) -> &str {
    let s = s.trim_start_matches('"');
    if let Some(p) = s.rfind('"') {
        &s[..p]
    } else {
        s
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn object_to_jsonld_value(obj: &str) -> String {
    if let Some(iri) = try_strip_angles(obj) {
        format!(r#"{{"@id": "{}"}}"#, json_escape(iri))
    } else if let Some((lit, lang)) = try_lang_literal(obj) {
        format!(
            r#"{{"@value": "{}", "@language": "{}"}}"#,
            json_escape(lit),
            lang
        )
    } else if let Some((lit, dt)) = try_typed_literal(obj) {
        format!(
            r#"{{"@value": "{}", "@type": "{}"}}"#,
            json_escape(lit),
            json_escape(dt)
        )
    } else {
        format!(r#"{{"@value": "{}"}}"#, json_escape(plain_literal(obj)))
    }
}
