//! `patpubrender` — parse USPTO patent XML/APS and render compact Markdown.
//!
//! Verbs:
//!   render [INPUT] [--output OUT]                 (always)
//!   shard write (--zip Z | --dir D) [--output D] [--limit N] [--jobs N]   (feature "ingest")
//!   shard read --shard F.zst (--key K | --offset N --length L) [--index F] [--output OUT]  (feature "shard")

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

/// Separator between documents when a directory is rendered to a single stream
/// (stdout or one file). Four newlines never occur inside a rendered doc — the
/// renderer collapses trailing blanks — so it is an unambiguous record boundary.
const DOC_SEPARATOR: &str = "\n\n\n\n";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("render") => run_render(&args[1..]),
        Some("shard") => run_shard(&args[1..]),
        _ => usage_exit(),
    }
}

fn usage_exit() -> ! {
    eprintln!("Usage:");
    eprintln!("  patpubrender render [INPUT] [--output <path>] [--template <file>]");
    eprintln!("      INPUT: a file, a directory, or - / omitted for stdin");
    eprintln!("      file/stdin -> stdout (or --output FILE)");
    eprintln!("      directory  -> all docs to stdout, or one .md per file into --output DIR");
    eprintln!("      --template: a .md template with {{frontmatter}}/{{title}}/{{abstract}}/");
    eprintln!("                  {{description}}/{{claims}}/{{body}} placeholders");
    eprintln!(
        "  patpubrender shard write (--zip <zip> | --dir <dir>) [--output <dir>] [--limit <n>] [--jobs <n>]"
    );
    eprintln!("      (requires the `ingest` feature)");
    eprintln!(
        "  patpubrender shard read --shard <file.zst> (--key <k> | --offset <n> --length <l>) [--index <file.idx>] [--output <path>]"
    );
    eprintln!("      (requires the `shard` feature)");
    process::exit(1);
}

fn fail(msg: impl AsRef<str>) -> ! {
    eprintln!("Error: {}", msg.as_ref());
    process::exit(1);
}

// ---------------------------------------------------------------------------
// render (tier 1)
// ---------------------------------------------------------------------------

fn run_render(args: &[String]) {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut template: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                i += 1;
                output = Some(
                    args.get(i)
                        .unwrap_or_else(|| fail("--output requires a path"))
                        .clone(),
                );
            }
            "--template" => {
                i += 1;
                let path = args
                    .get(i)
                    .unwrap_or_else(|| fail("--template requires a path"));
                template = Some(
                    fs::read_to_string(path)
                        .unwrap_or_else(|e| fail(format!("reading template '{path}': {e}"))),
                );
            }
            other if other.starts_with("--") => fail(format!("unknown flag '{other}'")),
            other => {
                if input.is_some() {
                    fail("multiple inputs specified");
                }
                input = Some(other.to_string());
            }
        }
        i += 1;
    }

    let template = template.as_deref();
    let is_dir = input
        .as_deref()
        .filter(|p| *p != "-")
        .is_some_and(|p| Path::new(p).is_dir());

    if is_dir {
        render_dir(
            Path::new(input.as_deref().unwrap()),
            output.as_deref(),
            template,
        );
    } else {
        render_single(input.as_deref(), output.as_deref(), template);
    }
}

fn render_single(input: Option<&str>, output: Option<&str>, template: Option<&str>) {
    let xml = match input {
        Some(path) if path != "-" => {
            fs::read_to_string(path).unwrap_or_else(|e| fail(format!("reading '{path}': {e}")))
        }
        _ => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .unwrap_or_else(|e| fail(format!("reading stdin: {e}")));
            buf
        }
    };

    if xml.trim().is_empty() {
        fail("empty input");
    }

    let md = render_xml(&xml, input.filter(|p| *p != "-"), template);

    match output {
        Some(path) => {
            fs::write(path, &md).unwrap_or_else(|e| fail(format!("writing '{path}': {e}")));
            eprintln!("Wrote {path}");
        }
        None => write_stdout(md.as_bytes()),
    }
}

fn render_dir(dir: &Path, output: Option<&str>, template: Option<&str>) {
    let mut xml_files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| fail(format!("reading directory '{}': {e}", dir.display())))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_xml_extension(path))
        .collect();
    xml_files.sort();

    if xml_files.is_empty() {
        fail(format!("no .xml files found in '{}'", dir.display()));
    }

    match output {
        // No --output: concatenate every doc to stdout, separated unambiguously.
        None => {
            let docs: Vec<String> = xml_files
                .iter()
                .map(|path| render_xml(&read_file(path), Some(&path.to_string_lossy()), template))
                .collect();
            write_stdout(docs.join(DOC_SEPARATOR).as_bytes());
        }
        // --output DIR: fan out to one .md per input file.
        Some(out_dir) => {
            let out_dir = Path::new(out_dir);
            fs::create_dir_all(out_dir)
                .unwrap_or_else(|e| fail(format!("creating '{}': {e}", out_dir.display())));
            for path in &xml_files {
                let md = render_xml(&read_file(path), Some(&path.to_string_lossy()), template);
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("document");
                let out_path = out_dir.join(format!("{stem}.md"));
                fs::write(&out_path, md)
                    .unwrap_or_else(|e| fail(format!("writing '{}': {e}", out_path.display())));
                eprintln!("Wrote {}", out_path.display());
            }
        }
    }
}

fn render_xml(xml: &str, label: Option<&str>, template: Option<&str>) -> String {
    let doc = patpubrender::parse_patent_xml(xml).unwrap_or_else(|e| {
        let where_ = label.map(|l| format!(" from '{l}'")).unwrap_or_default();
        fail(format!("parsing XML{where_}: {e}"))
    });
    if let Some(label) = label {
        eprintln!("{label}: detected {:?}", doc.source_format);
    }
    match template {
        Some(template) => patpubrender::render_markdown_with_template(&doc, template)
            .unwrap_or_else(|e| fail(format!("template: {e}"))),
        None => patpubrender::render_markdown(&doc),
    }
}

fn read_file(path: &Path) -> String {
    let xml = fs::read_to_string(path)
        .unwrap_or_else(|e| fail(format!("reading '{}': {e}", path.display())));
    if xml.trim().is_empty() {
        fail(format!("empty input '{}'", path.display()));
    }
    xml
}

fn has_xml_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xml"))
}

fn write_stdout(bytes: &[u8]) {
    let mut out = io::stdout();
    out.write_all(bytes)
        .unwrap_or_else(|e| fail(format!("writing stdout: {e}")));
    let _ = out.write_all(b"\n");
}

// ---------------------------------------------------------------------------
// shard (tiers 2 & 3 — cfg-gated)
// ---------------------------------------------------------------------------

fn run_shard(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("write") => shard_write(&args[1..]),
        Some("read") => shard_read(&args[1..]),
        _ => {
            eprintln!("Usage: patpubrender shard (write | read) ...");
            process::exit(1);
        }
    }
}

#[cfg(not(feature = "ingest"))]
fn shard_write(_args: &[String]) -> ! {
    fail("`shard write` requires the `ingest` feature (rebuild with --features ingest)");
}

#[cfg(feature = "ingest")]
fn shard_write(args: &[String]) {
    let mut zip: Option<String> = None;
    let mut dir: Option<String> = None;
    let mut output: Option<String> = None;
    let mut limit: Option<usize> = None;
    let mut jobs: Option<usize> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--zip" => {
                i += 1;
                zip = Some(arg(args, i, "--zip"));
            }
            "--dir" => {
                i += 1;
                dir = Some(arg(args, i, "--dir"));
            }
            "--output" => {
                i += 1;
                output = Some(arg(args, i, "--output"));
            }
            "--limit" => {
                i += 1;
                limit = Some(parse_count(&arg(args, i, "--limit"), "--limit"));
            }
            "--jobs" => {
                i += 1;
                jobs = Some(parse_count(&arg(args, i, "--jobs"), "--jobs"));
            }
            other => fail(format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    let out_dir = output.unwrap_or_else(|| ".".to_string());

    match (zip, dir) {
        (Some(_), Some(_)) => fail("--zip and --dir are mutually exclusive"),
        (Some(zip), None) => write_one_shard(&zip, &out_dir, limit),
        (None, Some(dir)) => write_many_shards(&dir, &out_dir, limit, jobs),
        (None, None) => fail("one of --zip or --dir is required"),
    }
}

#[cfg(feature = "ingest")]
fn write_one_shard(zip: &str, out_dir: &str, limit: Option<usize>) {
    match patpubrender::ingest::render_shard_from_zip(zip, out_dir, limit) {
        Ok(stats) => eprintln!(
            "shard write: {} written, {} supplemental, {} unsupported (fixable), {} malformed -> {} + {}",
            stats.docs_written,
            stats.supplemental_skipped,
            stats.unsupported_skipped,
            stats.malformed_skipped,
            stats.zst_path,
            stats.idx_path,
        ),
        Err(e) => fail(e.to_string()),
    }
}

#[cfg(feature = "ingest")]
fn write_many_shards(input_dir: &str, out_dir: &str, limit: Option<usize>, jobs: Option<usize>) {
    use rayon::prelude::*;

    let mut zips = Vec::new();
    collect_zips(Path::new(input_dir), &mut zips)
        .unwrap_or_else(|e| fail(format!("scanning '{input_dir}': {e}")));
    zips.sort();
    if zips.is_empty() {
        fail(format!("no *.zip files found under '{input_dir}'"));
    }

    if let Some(n) = jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .unwrap_or_else(|e| fail(format!("configuring thread pool: {e}")));
    }

    let total = zips.len();
    eprintln!("shard write: {total} zip(s) -> {out_dir}");
    let result: Result<(), String> = zips.par_iter().try_for_each(|zip| {
        let zip_str = zip.to_string_lossy();
        let stats = patpubrender::ingest::render_shard_from_zip(&zip_str, out_dir, limit)
            .map_err(|e| format!("{zip_str}: {e}"))?;
        eprintln!(
            "  {} -> {} written, {} supplemental",
            zip.file_stem().and_then(|s| s.to_str()).unwrap_or("?"),
            stats.docs_written,
            stats.supplemental_skipped,
        );
        Ok(())
    });
    if let Err(e) = result {
        fail(format!("aborted on first failure: {e}"));
    }
    eprintln!("shard write: done ({total} zips).");
}

#[cfg(feature = "ingest")]
fn collect_zips(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_zips(&path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
        {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(not(feature = "shard"))]
fn shard_read(_args: &[String]) -> ! {
    fail("`shard read` requires the `shard` feature (rebuild with --features shard)");
}

#[cfg(feature = "shard")]
fn shard_read(args: &[String]) {
    let mut shard: Option<String> = None;
    let mut index: Option<String> = None;
    let mut key: Option<String> = None;
    let mut offset: Option<u64> = None;
    let mut length: Option<u64> = None;
    let mut output: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--shard" => {
                i += 1;
                shard = Some(arg(args, i, "--shard"));
            }
            "--index" => {
                i += 1;
                index = Some(arg(args, i, "--index"));
            }
            "--key" => {
                i += 1;
                key = Some(arg(args, i, "--key"));
            }
            "--offset" => {
                i += 1;
                offset = Some(parse_u64(&arg(args, i, "--offset"), "--offset"));
            }
            "--length" => {
                i += 1;
                length = Some(parse_u64(&arg(args, i, "--length"), "--length"));
            }
            "--output" => {
                i += 1;
                output = Some(arg(args, i, "--output"));
            }
            other => fail(format!("unknown flag '{other}'")),
        }
        i += 1;
    }

    let shard_path = shard.unwrap_or_else(|| fail("--shard is required"));
    let shard_path = Path::new(&shard_path);

    let (offset, length) = match (key, offset, length) {
        // Resolve a doc key through the index (default: <shard-stem>.idx).
        (Some(key), _, _) => {
            let idx_path = index
                .map(PathBuf::from)
                .unwrap_or_else(|| shard_path.with_extension("idx"));
            let stem = shard_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let idx_text = fs::read_to_string(&idx_path)
                .unwrap_or_else(|e| fail(format!("reading index '{}': {e}", idx_path.display())));
            let entries = patpubrender::shard::parse_shard_index(stem, &idx_text)
                .unwrap_or_else(|e| fail(e.to_string()));
            let entry = entries
                .iter()
                .find(|e| e.doc_key == key)
                .unwrap_or_else(|| {
                    fail(format!("key '{key}' not found in '{}'", idx_path.display()))
                });
            (entry.pointer.offset, entry.pointer.length)
        }
        // Raw frame coordinates.
        (None, Some(off), Some(len)) => (off, len),
        (None, _, _) => fail("provide --key, or both --offset and --length"),
    };

    let bytes = patpubrender::shard::read_frame(shard_path, offset, length)
        .unwrap_or_else(|e| fail(format!("reading frame: {e}")));

    match output {
        Some(path) => {
            fs::write(&path, &bytes).unwrap_or_else(|e| fail(format!("writing '{path}': {e}")));
            eprintln!("Wrote {path}");
        }
        None => write_stdout(&bytes),
    }
}

// ---------------------------------------------------------------------------
// arg helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn arg(args: &[String], i: usize, flag: &str) -> String {
    args.get(i)
        .unwrap_or_else(|| fail(format!("{flag} requires a value")))
        .clone()
}

#[allow(dead_code)]
fn parse_count(value: &str, flag: &str) -> usize {
    let n = value
        .parse::<usize>()
        .unwrap_or_else(|_| fail(format!("{flag} must be a positive integer")));
    if n == 0 {
        fail(format!("{flag} must be greater than zero"));
    }
    n
}

#[allow(dead_code)]
fn parse_u64(value: &str, flag: &str) -> u64 {
    value
        .parse::<u64>()
        .unwrap_or_else(|_| fail(format!("{flag} must be a non-negative integer")))
}
