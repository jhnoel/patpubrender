//! Shards — the addressable compressed archive for rendered documents.
//!
//! A shard is a pair of files: `<stem>.zst` holds one independent zstd frame
//! per document (frame independence is what enables random access), and
//! `<stem>.idx` is a TSV of `doc_key \t offset \t length` rows pointing into it.
//!
//! This module is one surface covering the whole lifecycle:
//! - **codec** — [`ShardWriter`] / [`ShardReader`] / [`read_frame`] and the
//!   index types, the single owner of the on-disk format.
//! - **ingest** — [`render_shard_from_zip`], which renders a USPTO weekly bulk
//!   ZIP into a shard plus a `.biblio.jsonl` sidecar and `.manifest.json`.

mod codec;
mod ingest;

pub use codec::{
    SHARD_FORMAT_VERSION, SHARD_ZSTD_LEVEL, ShardIndexEntry, ShardPointer, ShardReader,
    ShardWriter, parse_shard_index, read_frame, shard_path,
};
pub use ingest::{ShardStats, render_shard_from_zip};
