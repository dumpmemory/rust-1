//! This module defines a generic file format that allows to check if a given
//! file generated by incremental compilation was generated by a compatible
//! compiler version. This file format is used for the on-disk version of the
//! dependency graph and the exported metadata hashes.
//!
//! In practice "compatible compiler version" means "exactly the same compiler
//! version", since the header encodes the git commit hash of the compiler.
//! Since we can always just ignore the incremental compilation cache and
//! compiler versions don't change frequently for the typical user, being
//! conservative here practically has no downside.

use crate::errors;
use rustc_data_structures::memmap::Mmap;
use rustc_serialize::opaque::{FileEncodeResult, FileEncoder};
use rustc_serialize::Encoder;
use rustc_session::Session;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// The first few bytes of files generated by incremental compilation.
const FILE_MAGIC: &[u8] = b"RSIC";

/// Change this if the header format changes.
const HEADER_FORMAT_VERSION: u16 = 0;

pub(crate) fn write_file_header(stream: &mut FileEncoder, sess: &Session) {
    stream.emit_raw_bytes(FILE_MAGIC);
    stream
        .emit_raw_bytes(&[(HEADER_FORMAT_VERSION >> 0) as u8, (HEADER_FORMAT_VERSION >> 8) as u8]);

    let rustc_version = rustc_version(sess.is_nightly_build(), sess.cfg_version);
    assert_eq!(rustc_version.len(), (rustc_version.len() as u8) as usize);
    stream.emit_raw_bytes(&[rustc_version.len() as u8]);
    stream.emit_raw_bytes(rustc_version.as_bytes());
}

pub(crate) fn save_in<F>(sess: &Session, path_buf: PathBuf, name: &str, encode: F)
where
    F: FnOnce(FileEncoder) -> FileEncodeResult,
{
    debug!("save: storing data in {}", path_buf.display());

    // Delete the old file, if any.
    // Note: It's important that we actually delete the old file and not just
    // truncate and overwrite it, since it might be a shared hard-link, the
    // underlying data of which we don't want to modify.
    //
    // We have to ensure we have dropped the memory maps to this file
    // before performing this removal.
    match fs::remove_file(&path_buf) {
        Ok(()) => {
            debug!("save: remove old file");
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => (),
        Err(err) => sess.dcx().emit_fatal(errors::DeleteOld { name, path: path_buf, err }),
    }

    let mut encoder = match FileEncoder::new(&path_buf) {
        Ok(encoder) => encoder,
        Err(err) => sess.dcx().emit_fatal(errors::CreateNew { name, path: path_buf, err }),
    };

    write_file_header(&mut encoder, sess);

    match encode(encoder) {
        Ok(position) => {
            sess.prof.artifact_size(
                &name.replace(' ', "_"),
                path_buf.file_name().unwrap().to_string_lossy(),
                position as u64,
            );
            debug!("save: data written to disk successfully");
        }
        Err((path, err)) => sess.dcx().emit_fatal(errors::WriteNew { name, path, err }),
    }
}

/// Reads the contents of a file with a file header as defined in this module.
///
/// - Returns `Ok(Some(data, pos))` if the file existed and was generated by a
///   compatible compiler version. `data` is the entire contents of the file
///   and `pos` points to the first byte after the header.
/// - Returns `Ok(None)` if the file did not exist or was generated by an
///   incompatible version of the compiler.
/// - Returns `Err(..)` if some kind of IO error occurred while reading the
///   file.
pub fn read_file(
    path: &Path,
    report_incremental_info: bool,
    is_nightly_build: bool,
    cfg_version: &'static str,
) -> io::Result<Option<(Mmap, usize)>> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    // SAFETY: This process must not modify nor remove the backing file while the memory map lives.
    // For the dep-graph and the work product index, it is as soon as the decoding is done.
    // For the query result cache, the memory map is dropped in save_dep_graph before calling
    // save_in and trying to remove the backing file.
    //
    // There is no way to prevent another process from modifying this file.
    let mmap = unsafe { Mmap::map(file) }?;

    let mut file = io::Cursor::new(&*mmap);

    // Check FILE_MAGIC
    {
        debug_assert!(FILE_MAGIC.len() == 4);
        let mut file_magic = [0u8; 4];
        file.read_exact(&mut file_magic)?;
        if file_magic != FILE_MAGIC {
            report_format_mismatch(report_incremental_info, path, "Wrong FILE_MAGIC");
            return Ok(None);
        }
    }

    // Check HEADER_FORMAT_VERSION
    {
        debug_assert!(::std::mem::size_of_val(&HEADER_FORMAT_VERSION) == 2);
        let mut header_format_version = [0u8; 2];
        file.read_exact(&mut header_format_version)?;
        let header_format_version =
            (header_format_version[0] as u16) | ((header_format_version[1] as u16) << 8);

        if header_format_version != HEADER_FORMAT_VERSION {
            report_format_mismatch(report_incremental_info, path, "Wrong HEADER_FORMAT_VERSION");
            return Ok(None);
        }
    }

    // Check RUSTC_VERSION
    {
        let mut rustc_version_str_len = [0u8; 1];
        file.read_exact(&mut rustc_version_str_len)?;
        let rustc_version_str_len = rustc_version_str_len[0] as usize;
        let mut buffer = vec![0; rustc_version_str_len];
        file.read_exact(&mut buffer)?;

        if buffer != rustc_version(is_nightly_build, cfg_version).as_bytes() {
            report_format_mismatch(report_incremental_info, path, "Different compiler version");
            return Ok(None);
        }
    }

    let post_header_start_pos = file.position() as usize;
    Ok(Some((mmap, post_header_start_pos)))
}

fn report_format_mismatch(report_incremental_info: bool, file: &Path, message: &str) {
    debug!("read_file: {}", message);

    if report_incremental_info {
        eprintln!(
            "[incremental] ignoring cache artifact `{}`: {}",
            file.file_name().unwrap().to_string_lossy(),
            message
        );
    }
}

/// A version string that hopefully is always different for compiler versions
/// with different encodings of incremental compilation artifacts. Contains
/// the Git commit hash.
fn rustc_version(nightly_build: bool, cfg_version: &'static str) -> Cow<'static, str> {
    if nightly_build {
        if let Ok(val) = env::var("RUSTC_FORCE_RUSTC_VERSION") {
            return val.into();
        }
    }

    cfg_version.into()
}
