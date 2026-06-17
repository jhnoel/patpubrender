//! Shard codec — the addressable compressed archive format for rendered docs.
//!
//! A shard is a pair of files: `<stem>.zst` holds one independent zstd frame
//! per document (frame independence is what makes random access possible), and
//! `<stem>.idx` is a TSV of `doc_key \t offset \t length` rows pointing into it.
//!
//! Write ([`ShardWriter`]) and read ([`ShardReader`] / [`read_frame`]) live in
//! this one module on purpose: the on-disk format has a single owner, so a
//! format change touches both sides in one diff. [`SHARD_FORMAT_VERSION`] is the
//! version the two sides agree on.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// On-disk shard format version. Bump when the frame/index layout changes;
/// the writer and reader move together because they live here together.
pub const SHARD_FORMAT_VERSION: u32 = 1;

/// zstd level for per-document frames.
///
/// Each document is its own independent frame (required for random-access
/// reads), so the level only trades write-time CPU against on-disk size;
/// decompression speed is level-independent, so reads are unaffected. Level 9
/// is the knee for patent Markdown — level 19 spends ~33x the CPU for only
/// ~17% smaller frames.
pub const SHARD_ZSTD_LEVEL: i32 = 9;

/// A located, length-delimited frame within a named shard's `.zst` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardPointer {
    pub shard: String,
    pub offset: u64,
    pub length: u64,
}

impl ShardPointer {
    pub fn new(shard: impl Into<String>, offset: u64, length: u64) -> Self {
        Self {
            shard: shard.into(),
            offset,
            length,
        }
    }

    /// Resolve this pointer's `.zst` path under `shards_root`.
    pub fn shard_path(&self, shards_root: &Path) -> PathBuf {
        shard_path(shards_root, &self.shard)
    }
}

/// One parsed `.idx` row: a doc key and where its frame lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardIndexEntry {
    pub doc_key: String,
    pub pointer: ShardPointer,
}

/// `<shards_root>/<shard>.zst`.
pub fn shard_path(shards_root: &Path, shard: &str) -> PathBuf {
    shards_root.join(format!("{}.zst", shard.trim()))
}

/// Parse a `.idx` TSV — one `doc_key \t offset \t length` row per line — into
/// entries, each pointer tagged with `shard`.
pub fn parse_shard_index(shard: &str, idx_text: &str) -> io::Result<Vec<ShardIndexEntry>> {
    let mut entries = Vec::new();
    for (line_number, line) in idx_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 {
            return Err(idx_err(line_number, "expected 3 tab-separated fields"));
        }
        let doc_key = parts[0].trim();
        if doc_key.is_empty() {
            return Err(idx_err(line_number, "empty doc key"));
        }
        let offset = parts[1]
            .parse::<u64>()
            .map_err(|e| idx_err(line_number, &format!("offset: {e}")))?;
        let length = parts[2]
            .parse::<u64>()
            .map_err(|e| idx_err(line_number, &format!("length: {e}")))?;
        entries.push(ShardIndexEntry {
            doc_key: doc_key.to_string(),
            pointer: ShardPointer::new(shard, offset, length),
        });
    }
    Ok(entries)
}

fn idx_err(line_number: usize, msg: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid shard index line {}: {msg}", line_number + 1),
    )
}

/// Streaming writer: appends one independent zstd frame per document to the
/// `.zst` and one row per document to the `.idx`, tracking byte offsets.
pub struct ShardWriter {
    shard: String,
    zst: File,
    idx: File,
    offset: u64,
}

impl ShardWriter {
    /// Create `<out_dir>/<stem>.zst` and `<out_dir>/<stem>.idx`.
    pub fn create(out_dir: &Path, stem: &str) -> io::Result<Self> {
        let zst = File::create(out_dir.join(format!("{stem}.zst")))?;
        let idx = File::create(out_dir.join(format!("{stem}.idx")))?;
        Ok(Self {
            shard: stem.to_string(),
            zst,
            idx,
            offset: 0,
        })
    }

    /// Compress `payload` as one frame, append it to the `.zst`, write the
    /// matching `.idx` row, and return the pointer just written.
    pub fn append(&mut self, doc_key: &str, payload: &[u8]) -> io::Result<ShardPointer> {
        let frame = zstd::encode_all(payload, SHARD_ZSTD_LEVEL)?;
        let length = frame.len() as u64;
        self.zst.write_all(&frame)?;
        writeln!(self.idx, "{doc_key}\t{}\t{length}", self.offset)?;
        let pointer = ShardPointer::new(self.shard.clone(), self.offset, length);
        self.offset += length;
        Ok(pointer)
    }
}

/// Read and decompress a single frame from a `.zst` file by absolute
/// `offset`/`length`. Stateless — use this when you already hold the shard path
/// (e.g. the CLI `shard read`).
pub fn read_frame(zst_path: &Path, offset: u64, length: u64) -> io::Result<Vec<u8>> {
    let mut file = File::open(zst_path)?;
    read_frame_from(&mut file, offset, length)
}

fn read_frame_from(file: &mut File, offset: u64, length: u64) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut blob = vec![0u8; length as usize];
    file.read_exact(&mut blob)?;
    zstd::decode_all(blob.as_slice())
}

/// Random-access reader rooted at a shards directory, caching one open file
/// handle per shard. Use this when resolving many [`ShardPointer`]s that name
/// shards by stem (e.g. a catalog store).
#[derive(Debug, Default)]
pub struct ShardReader {
    root: PathBuf,
    files: HashMap<String, File>,
}

impl ShardReader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            files: HashMap::new(),
        }
    }

    /// Read a frame and decode it as UTF-8 text.
    pub fn read_text(&mut self, pointer: &ShardPointer) -> io::Result<String> {
        let bytes = self.read_bytes(pointer)?;
        String::from_utf8(bytes).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("decompressed {} document was not UTF-8: {e}", pointer.shard),
            )
        })
    }

    /// Read and decompress the frame a pointer names.
    pub fn read_bytes(&mut self, pointer: &ShardPointer) -> io::Result<Vec<u8>> {
        let (offset, length) = (pointer.offset, pointer.length);
        let file = self.open_file(&pointer.shard)?;
        read_frame_from(file, offset, length)
    }

    fn open_file(&mut self, shard: &str) -> io::Result<&mut File> {
        if !self.files.contains_key(shard) {
            let file = File::open(shard_path(&self.root, shard))?;
            self.files.insert(shard.to_string(), file);
        }
        Ok(self.files.get_mut(shard).expect("shard file just inserted"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read_roundtrips_each_frame() {
        let dir = tempfile::tempdir().unwrap();
        let docs = [("US1", "alpha doc"), ("US2", "beta doc body")];

        let mut writer = ShardWriter::create(dir.path(), "wk01").unwrap();
        let mut pointers = Vec::new();
        for (key, body) in &docs {
            pointers.push((key, writer.append(key, body.as_bytes()).unwrap()));
        }
        drop(writer);

        // Read back via the index.
        let idx_text = std::fs::read_to_string(dir.path().join("wk01.idx")).unwrap();
        let entries = parse_shard_index("wk01", &idx_text).unwrap();
        assert_eq!(entries.len(), 2);

        let mut reader = ShardReader::new(dir.path());
        for ((_, body), entry) in docs.iter().zip(&entries) {
            assert_eq!(&reader.read_text(&entry.pointer).unwrap(), body);
        }

        // And via stateless read_frame against the direct path.
        let zst = dir.path().join("wk01.zst");
        for ((_, body), (_, ptr)) in docs.iter().zip(&pointers) {
            let bytes = read_frame(&zst, ptr.offset, ptr.length).unwrap();
            assert_eq!(&String::from_utf8(bytes).unwrap(), body);
        }
    }
}
