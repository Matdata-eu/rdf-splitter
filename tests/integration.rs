//! Integration tests – exercises the `rdfsplitter` binary end-to-end.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("rdfsplitter"))
}

/// Absolute path to a test fixture file.
fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
}

fn count_files(dir: &TempDir) -> usize {
    fs::read_dir(dir.path()).unwrap().count()
}

fn out(dir: &TempDir) -> String {
    dir.path().to_str().unwrap().to_owned()
}

// ── help / version ────────────────────────────────────────────────────────────

#[test]
fn help_exits_success() {
    cmd().arg("--help").assert().success();
}

#[test]
fn version_exits_success() {
    cmd().arg("--version").assert().success();
}

// ── N-Triples ─────────────────────────────────────────────────────────────────

#[test]
fn nt_chunk_size_produces_correct_file_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-n", "3", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    // 10 triples / 3 per chunk → 4 files (3+3+3+1)
    assert_eq!(count_files(&dir), 4);
}

#[test]
fn nt_file_count_produces_correct_file_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-c", "2", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    // 10 triples split into exactly 2 files
    assert_eq!(count_files(&dir), 2);
}

#[test]
fn nt_file_count_single_file() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-c", "1", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    assert_eq!(count_files(&dir), 1);
}

#[test]
fn nt_output_files_have_nt_extension() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    let files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!files.is_empty());
    assert!(files.iter().all(|f| f.ends_with(".nt")));
}

#[test]
fn nt_output_files_named_with_stem_and_index() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-n", "5", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    assert!(dir.path().join("small_0000.nt").exists());
    assert!(dir.path().join("small_0001.nt").exists());
}

#[test]
fn nt_first_chunk_has_correct_triple_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-n", "4", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    let content = fs::read_to_string(dir.path().join("small_0000.nt")).unwrap();
    let triple_lines = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert_eq!(triple_lines, 4);
}

#[test]
fn nt_last_chunk_contains_remainder() {
    let dir = TempDir::new().unwrap();
    // 10 triples / 3 → last chunk has 1 triple
    cmd()
        .args([&fixture("small.nt"), "-n", "3", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    let content = fs::read_to_string(dir.path().join("small_0003.nt")).unwrap();
    let triple_lines = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert_eq!(triple_lines, 1);
}

// ── Turtle ────────────────────────────────────────────────────────────────────

#[test]
fn ttl_chunk_size_produces_correct_file_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.ttl"), "-n", "3", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    assert_eq!(count_files(&dir), 4);
}

#[test]
fn ttl_output_files_have_ttl_extension() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.ttl"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    let files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(files.iter().all(|f| f.ends_with(".ttl")));
}

// ── N-Quads ───────────────────────────────────────────────────────────────────

#[test]
fn nq_chunk_size_produces_correct_file_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nq"), "-n", "3", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    assert_eq!(count_files(&dir), 4);
}

#[test]
fn nq_output_files_have_nq_extension() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nq"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    let files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(files.iter().all(|f| f.ends_with(".nq")));
}

// ── TriG ──────────────────────────────────────────────────────────────────────

#[test]
fn trig_chunk_size_produces_correct_file_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.trig"), "-n", "3", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    assert_eq!(count_files(&dir), 4);
}

// ── RDF/XML ───────────────────────────────────────────────────────────────────

#[test]
fn rdf_chunk_size_produces_correct_file_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.rdf"), "-n", "3", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    assert_eq!(count_files(&dir), 4);
}

#[test]
fn rdf_output_files_have_rdf_extension() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.rdf"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    let files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(files.iter().all(|f| f.ends_with(".rdf")));
}

// ── JSON-LD ───────────────────────────────────────────────────────────────────

#[test]
fn jsonld_chunk_size_produces_correct_file_count() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.jsonld"), "-n", "3", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    assert_eq!(count_files(&dir), 4);
}

#[test]
fn jsonld_output_files_have_jsonld_extension() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.jsonld"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    let files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(files.iter().all(|f| f.ends_with(".jsonld")));
}

// ── output directory / force ──────────────────────────────────────────────────

#[test]
fn force_creates_missing_output_directory() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("brand_new_dir");
    assert!(!sub.exists());
    cmd()
        .args([
            &fixture("small.nt"),
            "-n", "10",
            "-o", sub.to_str().unwrap(),
            "-f",
        ])
        .assert()
        .success();
    assert!(sub.exists());
}

#[test]
fn no_force_fails_when_output_directory_is_missing() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("nonexistent");
    cmd()
        .args([&fixture("small.nt"), "-n", "10", "-o", sub.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn no_force_fails_when_output_file_already_exists() {
    let dir = TempDir::new().unwrap();
    // First run creates files
    cmd()
        .args([&fixture("small.nt"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    // Second run without -f should fail because outputs exist
    cmd()
        .args([&fixture("small.nt"), "-n", "10", "-o", &out(&dir)])
        .assert()
        .failure();
}

#[test]
fn force_overwrites_existing_output_files() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    // Second run with -f must succeed
    cmd()
        .args([&fixture("small.nt"), "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .success();
}

// ── verbose output ────────────────────────────────────────────────────────────

#[test]
fn verbose_flag_prints_debug_info() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([&fixture("small.nt"), "-n", "10", "-o", &out(&dir), "-f", "-v"])
        .assert()
        .success()
        .stderr(predicate::str::contains("writing chunk"));
}

// ── recursive ─────────────────────────────────────────────────────────────────

#[test]
fn recursive_finds_nt_files_in_subdirectory() {
    let dir = TempDir::new().unwrap();
    let fixtures_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    cmd()
        .args([&fixtures_dir, "-r", "-n", "100", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    // subdir/sub.nt has 6 triples in one chunk; plus all top-level fixtures → many files
    assert!(count_files(&dir) > 0);
    // The subdir fixture should produce sub_0000.nt
    assert!(dir.path().join("sub_0000.nt").exists());
}

#[test]
fn without_recursive_flag_subdir_is_not_walked() {
    let dir = TempDir::new().unwrap();
    let fixtures_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    // Pass the directory without -r; tool should still walk top-level only
    // (top-level RDF files should still be processed)
    cmd()
        .args([&fixtures_dir, "-n", "100", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    // sub.nt should NOT be present because -r was omitted
    assert!(!dir.path().join("sub_0000.nt").exists());
}

// ── glob patterns ─────────────────────────────────────────────────────────────

#[test]
fn glob_star_nt_matches_all_nt_fixtures() {
    let dir = TempDir::new().unwrap();
    let pat = format!(
        "{}/tests/fixtures/*.nt",
        env!("CARGO_MANIFEST_DIR")
    );
    cmd()
        .args([&pat, "-n", "100", "-o", &out(&dir), "-f"])
        .assert()
        .success();
    // small.nt → 1 chunk
    assert!(dir.path().join("small_0000.nt").exists());
}

// ── conflicting options ───────────────────────────────────────────────────────

#[test]
fn chunk_size_and_file_count_are_mutually_exclusive() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([
            &fixture("small.nt"),
            "-n", "10",
            "-c", "2",
            "-o", &out(&dir),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ── nonexistent input ─────────────────────────────────────────────────────────

#[test]
fn nonexistent_input_file_exits_with_failure() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args(["/no/such/file.nt", "-n", "10", "-o", &out(&dir), "-f"])
        .assert()
        .failure();
}

// ── multiple input files ──────────────────────────────────────────────────────

#[test]
fn multiple_input_files_all_split() {
    let dir = TempDir::new().unwrap();
    cmd()
        .args([
            &fixture("small.nt"),
            &fixture("small.ttl"),
            "-n", "5",
            "-o", &out(&dir),
            "-f",
        ])
        .assert()
        .success();
    // each has 10 triples / 5 per chunk → 2 files each → 4 total
    assert_eq!(count_files(&dir), 4);
}
