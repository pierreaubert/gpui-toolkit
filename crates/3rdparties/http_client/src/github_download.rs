use std::{
    path::{Path, PathBuf},
    pin::Pin,
    task::Poll,
};

use anyhow::{Context, Result};
use async_compression::futures::bufread::{BzDecoder, GzipDecoder};
use futures::{AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt, io::BufReader};
use sha2::{Digest, Sha256};

use crate::{HttpClient, github::AssetKind};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct GithubBinaryMetadata {
    pub metadata_version: u64,
    pub digest: Option<String>,
}

impl GithubBinaryMetadata {
    pub async fn read_from_file(metadata_path: &Path) -> Result<GithubBinaryMetadata> {
        let metadata_content = async_fs::read_to_string(metadata_path)
            .await
            .with_context(|| format!("reading metadata file at {metadata_path:?}"))?;
        serde_json::from_str(&metadata_content)
            .with_context(|| format!("parsing metadata file at {metadata_path:?}"))
    }

    pub async fn write_to_file(&self, metadata_path: &Path) -> Result<()> {
        let metadata_content = serde_json::to_string(self)
            .with_context(|| format!("serializing metadata for {metadata_path:?}"))?;
        async_fs::write(metadata_path, metadata_content.as_bytes())
            .await
            .with_context(|| format!("writing metadata file at {metadata_path:?}"))?;
        Ok(())
    }
}

pub async fn download_server_binary(
    http_client: &dyn HttpClient,
    url: &str,
    digest: Option<&str>,
    destination_path: &Path,
    asset_kind: AssetKind,
) -> Result<(), anyhow::Error> {
    log::info!("downloading github artifact from {url}");
    let Some(destination_parent) = destination_path.parent() else {
        anyhow::bail!("destination path has no parent: {destination_path:?}");
    };

    let staging_path = staging_path(destination_parent, asset_kind)?;
    let mut response = http_client
        .get(url, Default::default(), true)
        .await
        .with_context(|| format!("downloading release from {url}"))?;
    let body = response.body_mut();

    if let Err(err) = extract_to_staging(body, digest, url, &staging_path, asset_kind).await {
        cleanup_staging_path(&staging_path, asset_kind).await;
        return Err(err);
    }

    if let Err(err) = finalize_download(&staging_path, destination_path).await {
        cleanup_staging_path(&staging_path, asset_kind).await;
        return Err(err);
    }

    Ok(())
}

pub async fn download_server_raw_binary(
    http_client: &dyn HttpClient,
    url: &str,
    digest: Option<&str>,
    destination_path: &Path,
    binary_file_name: &str,
) -> Result<(), anyhow::Error> {
    log::info!("downloading raw binary from {url}");
    let Some(destination_parent) = destination_path.parent() else {
        anyhow::bail!("destination path has no parent: {destination_path:?}");
    };

    let staging_path = staging_dir_path(destination_parent)?;
    let result = async {
        let mut response = http_client
            .get(url, Default::default(), true)
            .await
            .with_context(|| format!("downloading release from {url}"))?;

        let binary_path = staging_path.join(binary_file_name);
        let mut writer = HashingWriter {
            writer: async_fs::File::create(&binary_path)
                .await
                .with_context(|| format!("creating a file {binary_path:?} for {url}"))?,
            hasher: Sha256::new(),
        };
        futures::io::copy(&mut BufReader::new(response.body_mut()), &mut writer)
            .await
            .with_context(|| format!("saving binary contents from {url}"))?;
        let asset_sha_256 = writer
            .finish()
            .await
            .with_context(|| format!("flushing binary contents for {url}"))?;

        if let Some(expected_sha_256) = digest {
            anyhow::ensure!(
                asset_sha_256 == expected_sha_256,
                "{url} asset got SHA-256 mismatch. Expected: {expected_sha_256}, Got: {asset_sha_256}",
            );
        }

        util::fs::make_file_executable(&binary_path)
            .await
            .with_context(|| format!("marking {binary_path:?} as executable"))?;
        finalize_download(&staging_path, destination_path).await
    }
    .await;

    if let Err(err) = result {
        if let Err(err) = async_fs::remove_dir_all(&staging_path).await {
            log::warn!("failed to remove staging directory {staging_path:?}: {err:?}");
        }
        return Err(err);
    }

    Ok(())
}

async fn extract_to_staging(
    body: impl AsyncRead + Unpin,
    digest: Option<&str>,
    url: &str,
    staging_path: &Path,
    asset_kind: AssetKind,
) -> Result<()> {
    match digest {
        Some(expected_sha_256) => {
            let temp_asset_file = tempfile::NamedTempFile::new()
                .with_context(|| format!("creating a temporary file for {url}"))?;
            let (temp_asset_file, _temp_guard) = temp_asset_file.into_parts();
            let mut writer = HashingWriter {
                writer: async_fs::File::from(temp_asset_file),
                hasher: Sha256::new(),
            };
            futures::io::copy(&mut BufReader::new(body), &mut writer)
                .await
                .with_context(|| {
                    format!("saving archive contents into the temporary file for {url}")
                })?;
            let asset_sha_256 = sha256_hex(writer.hasher);

            anyhow::ensure!(
                asset_sha_256 == expected_sha_256,
                "{url} asset got SHA-256 mismatch. Expected: {expected_sha_256}, Got: {asset_sha_256}",
            );
            writer
                .writer
                .seek(std::io::SeekFrom::Start(0))
                .await
                .with_context(|| format!("seeking temporary file for {url}"))?;
            stream_file_archive(&mut writer.writer, url, staging_path, asset_kind)
                .await
                .with_context(|| {
                    format!("extracting downloaded asset for {url} into {staging_path:?}")
                })?;
        }
        None => {
            stream_response_archive(body, url, staging_path, asset_kind)
                .await
                .with_context(|| {
                    format!("extracting response for asset {url} into {staging_path:?}")
                })?;
        }
    }
    Ok(())
}

fn staging_dir_path(parent: &Path) -> Result<PathBuf> {
    let dir = tempfile::Builder::new()
        .prefix(".tmp-github-download-")
        .tempdir_in(parent)
        .with_context(|| format!("creating staging directory in {parent:?}"))?;
    Ok(dir.keep())
}

fn staging_path(parent: &Path, asset_kind: AssetKind) -> Result<PathBuf> {
    match asset_kind {
        AssetKind::TarGz | AssetKind::TarBz2 | AssetKind::Zip => staging_dir_path(parent),
        AssetKind::Gz => {
            let path = tempfile::Builder::new()
                .prefix(".tmp-github-download-")
                .tempfile_in(parent)
                .with_context(|| format!("creating staging file in {parent:?}"))?
                .into_temp_path()
                .keep()
                .with_context(|| format!("persisting staging file in {parent:?}"))?;
            Ok(path)
        }
    }
}

async fn cleanup_staging_path(staging_path: &Path, asset_kind: AssetKind) {
    match asset_kind {
        AssetKind::TarGz | AssetKind::TarBz2 | AssetKind::Zip => {
            if let Err(err) = async_fs::remove_dir_all(staging_path).await {
                log::warn!("failed to remove staging directory {staging_path:?}: {err:?}");
            }
        }
        AssetKind::Gz => {
            if let Err(err) = async_fs::remove_file(staging_path).await {
                log::warn!("failed to remove staging file {staging_path:?}: {err:?}");
            }
        }
    }
}

async fn finalize_download(staging_path: &Path, destination_path: &Path) -> Result<()> {
    _ = async_fs::remove_dir_all(destination_path).await;
    async_fs::rename(staging_path, destination_path)
        .await
        .with_context(|| format!("renaming {staging_path:?} to {destination_path:?}"))?;
    Ok(())
}

async fn stream_response_archive(
    response: impl AsyncRead + Unpin,
    url: &str,
    destination_path: &Path,
    asset_kind: AssetKind,
) -> Result<()> {
    match asset_kind {
        AssetKind::TarGz => extract_tar_gz(destination_path, url, response).await?,
        AssetKind::TarBz2 => extract_tar_bz2(destination_path, url, response).await?,
        AssetKind::Gz => extract_gz(destination_path, url, response).await?,
        AssetKind::Zip => {
            util::archive::extract_zip(destination_path, response).await?;
        }
    };
    Ok(())
}

async fn stream_file_archive(
    file_archive: impl AsyncRead + AsyncSeek + Unpin,
    url: &str,
    destination_path: &Path,
    asset_kind: AssetKind,
) -> Result<()> {
    match asset_kind {
        AssetKind::TarGz => extract_tar_gz(destination_path, url, file_archive).await?,
        AssetKind::TarBz2 => extract_tar_bz2(destination_path, url, file_archive).await?,
        AssetKind::Gz => extract_gz(destination_path, url, file_archive).await?,
        #[cfg(not(windows))]
        AssetKind::Zip => {
            util::archive::extract_seekable_zip(destination_path, file_archive).await?;
        }
        #[cfg(windows)]
        AssetKind::Zip => {
            util::archive::extract_zip(destination_path, file_archive).await?;
        }
    };
    Ok(())
}

async fn extract_tar_gz(
    destination_path: &Path,
    url: &str,
    from: impl AsyncRead + Unpin,
) -> Result<(), anyhow::Error> {
    let decompressed_bytes = GzipDecoder::new(BufReader::new(from));
    unpack_tar_archive(destination_path, url, decompressed_bytes).await?;
    Ok(())
}

async fn extract_tar_bz2(
    destination_path: &Path,
    url: &str,
    from: impl AsyncRead + Unpin,
) -> Result<(), anyhow::Error> {
    let decompressed_bytes = BzDecoder::new(BufReader::new(from));
    unpack_tar_archive(destination_path, url, decompressed_bytes).await?;
    Ok(())
}

async fn unpack_tar_archive(
    destination_path: &Path,
    url: &str,
    archive_bytes: impl AsyncRead + Unpin,
) -> Result<(), anyhow::Error> {
    // We don't need to set the modified time. It's irrelevant to downloaded
    // archive verification, and some filesystems return errors when asked to
    // apply it after extraction.
    let archive = async_tar::ArchiveBuilder::new(async_compat::Compat::new(archive_bytes))
        .set_preserve_mtime(false)
        .build();
    async_compat::Compat::new(archive.unpack(&destination_path))
        .await
        .with_context(|| format!("extracting {url} to {destination_path:?}"))?;
    Ok(())
}

async fn extract_gz(
    destination_path: &Path,
    url: &str,
    from: impl AsyncRead + Unpin,
) -> Result<(), anyhow::Error> {
    let mut decompressed_bytes = GzipDecoder::new(BufReader::new(from));
    let mut file = async_fs::File::create(&destination_path)
        .await
        .with_context(|| {
            format!("creating a file {destination_path:?} for a download from {url}")
        })?;
    futures::io::copy(&mut decompressed_bytes, &mut file)
        .await
        .with_context(|| format!("extracting {url} to {destination_path:?}"))?;
    Ok(())
}

struct HashingWriter<W: AsyncWrite + Unpin> {
    writer: W,
    hasher: Sha256,
}

fn hex_nibble(nibble: u8) -> char {
    (match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }) as char
}

/// Hex SHA-256 digest of a finished hasher. sha2 0.11 no longer formats
/// digests as hex, so this encodes the finalized bytes manually.
fn sha256_hex(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            hex.push(hex_nibble(byte >> 4));
            hex.push(hex_nibble(byte & 0x0f));
            hex
        })
}

impl<W: AsyncWrite + Unpin> HashingWriter<W> {
    /// Closes and drops the inner writer, returning the hex SHA-256 digest of
    /// everything written.
    ///
    /// Taking `self` by value guarantees the writer is dropped before this
    /// returns. For file writers this releases the OS handle, which Windows
    /// requires before an ancestor directory can be renamed or deleted; note
    /// that closing alone is not enough, as `async_fs::File` holds its handle
    /// until dropped.
    async fn finish(mut self) -> std::io::Result<String> {
        self.writer.close().await?;
        drop(self.writer);
        Ok(sha256_hex(self.hasher))
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for HashingWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        match Pin::new(&mut self.writer).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => {
                self.hasher.update(&buf[..n]);
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        Pin::new(&mut self.writer).poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AsyncBody, Response};
    use futures::future::BoxFuture;
    use http::HeaderValue;
    use std::io::Write;
    use url::Url;

    const NORMAL_TAR: &[u8] = include_bytes!("../tests/data/normal.tar");
    const SYMLINK_ESCAPE_TAR: &[u8] = include_bytes!("../tests/data/symlink_escape.tar");
    const PAX_ESCAPE_TAR: &[u8] = include_bytes!("../tests/data/pax_escape.tar");

    fn compress_tar(kind: AssetKind, bytes: &[u8]) -> Vec<u8> {
        match kind {
            AssetKind::TarGz => {
                let mut encoder =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(bytes).unwrap();
                encoder.finish().unwrap()
            }
            AssetKind::TarBz2 => {
                let mut encoder =
                    bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
                encoder.write_all(bytes).unwrap();
                encoder.finish().unwrap()
            }
            _ => unreachable!("only tar variants have this fixture"),
        }
    }

    fn archive_with_first_path(bytes: &[u8], path: &Path) -> Vec<u8> {
        let mut archive = bytes.to_vec();
        let name = path.as_os_str().as_encoded_bytes();
        assert!(
            name.len() <= 100,
            "tar fixture path exceeds ustar name field"
        );
        archive[..100].fill(0);
        archive[..name.len()].copy_from_slice(name);
        archive[148..156].fill(b' ');
        let checksum: u32 = archive[..512].iter().map(|byte| u32::from(*byte)).sum();
        archive[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
        archive
    }

    struct StaticResponseClient {
        body: Vec<u8>,
    }

    impl HttpClient for StaticResponseClient {
        fn send(
            &self,
            _req: http::Request<AsyncBody>,
        ) -> BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
            let body = self.body.clone();
            Box::pin(async move {
                Ok(Response::builder()
                    .status(200)
                    .body(AsyncBody::from(body))
                    .unwrap())
            })
        }

        fn user_agent(&self) -> Option<&HeaderValue> {
            None
        }

        fn proxy(&self) -> Option<&Url> {
            None
        }
    }

    async fn assert_tar_download(kind: AssetKind, verify_digest: bool) {
        let temp_dir = tempfile::tempdir().unwrap();
        let destination = temp_dir.path().join("release");
        let body = compress_tar(kind, NORMAL_TAR);
        let expected_digest = sha256_hex(Sha256::new_with_prefix(&body));
        let client = StaticResponseClient { body };
        download_server_binary(
            &client,
            "https://example.com/release.tar",
            verify_digest.then_some(expected_digest.as_str()),
            &destination,
            kind,
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read(destination.join("bin/tool")).unwrap(),
            b"hello from archive\n"
        );
        assert_eq!(
            std::fs::read(destination.join("docs/readme.txt")).unwrap(),
            b"archive metadata\n"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                std::fs::metadata(destination.join("bin/tool"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o111,
                0
            );
        }
        assert_eq!(std::fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn tar_downloads_with_both_codecs_and_digest_modes_on_futures_and_smol() {
        for kind in [AssetKind::TarGz, AssetKind::TarBz2] {
            for verify_digest in [false, true] {
                futures::executor::block_on(assert_tar_download(kind, verify_digest));
                smol::block_on(smol::spawn(assert_tar_download(kind, verify_digest)));
            }
        }
    }

    async fn assert_tar_failure_cleans_staging(
        kind: AssetKind,
        body: Vec<u8>,
        digest: Option<&str>,
    ) {
        let temp_dir = tempfile::tempdir().unwrap();
        let destination = temp_dir.path().join("release");
        let client = StaticResponseClient { body };
        assert!(
            download_server_binary(
                &client,
                "https://example.com/release.tar",
                digest,
                &destination,
                kind
            )
            .await
            .is_err()
        );
        assert!(!destination.exists());
        assert_eq!(std::fs::read_dir(temp_dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn tar_corruption_truncation_and_digest_mismatch_leave_no_destination() {
        futures::executor::block_on(async {
            for kind in [AssetKind::TarGz, AssetKind::TarBz2] {
                assert_tar_failure_cleans_staging(kind, b"not a compressed archive".to_vec(), None)
                    .await;
                let mut truncated = compress_tar(kind, NORMAL_TAR);
                truncated.truncate(truncated.len() / 2);
                assert_tar_failure_cleans_staging(kind, truncated, None).await;
                assert_tar_failure_cleans_staging(
                    kind,
                    compress_tar(kind, NORMAL_TAR),
                    Some("0000000000000000000000000000000000000000000000000000000000000000"),
                )
                .await;
            }
        });
    }

    async fn assert_tar_path_confined(archive: &[u8], outside_relative: &Path) {
        let temp_dir = tempfile::tempdir().unwrap();
        let outside = temp_dir.path().join(outside_relative);
        std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
        let destination = temp_dir.path().join("release");
        let body = compress_tar(AssetKind::TarGz, archive);
        let client = StaticResponseClient { body };
        let result = download_server_binary(
            &client,
            "https://example.com/release.tar.gz",
            None,
            &destination,
            AssetKind::TarGz,
        )
        .await;
        match result {
            Ok(()) => {
                assert!(destination.is_dir(), "safe extraction did not finalize");
                if outside_relative == Path::new("outside/escape.txt") {
                    assert!(
                        std::fs::symlink_metadata(destination.join("link")).is_err(),
                        "archive retained a link outside its staging directory"
                    );
                }
            }
            Err(error) => {
                assert!(
                    format!("{error:#}").contains("extracting"),
                    "archive failed before extraction: {error:#}"
                );
                assert!(!destination.exists());
            }
        }
        assert!(
            !outside.exists(),
            "archive wrote outside its staging directory"
        );
    }

    #[test]
    fn tar_traversal_symlink_and_pax_entries_stay_within_staging() {
        futures::executor::block_on(async {
            assert!(
                SYMLINK_ESCAPE_TAR
                    .windows(b"../outside".len())
                    .any(|part| part == b"../outside")
            );
            assert!(
                SYMLINK_ESCAPE_TAR
                    .windows(b"link/escape.txt".len())
                    .any(|part| part == b"link/escape.txt")
            );
            assert!(
                PAX_ESCAPE_TAR
                    .windows(b"../pax-escape.txt".len())
                    .any(|part| part == b"../pax-escape.txt")
            );
            assert_tar_path_confined(
                &archive_with_first_path(NORMAL_TAR, Path::new("../escape.txt")),
                Path::new("escape.txt"),
            )
            .await;
            assert_tar_path_confined(PAX_ESCAPE_TAR, Path::new("pax-escape.txt")).await;
            assert_tar_path_confined(SYMLINK_ESCAPE_TAR, Path::new("outside/escape.txt")).await;
            #[cfg(unix)]
            {
                let temp_dir = tempfile::tempdir().unwrap();
                let outside = temp_dir.path().join("absolute-escape.txt");
                let destination = temp_dir.path().join("release");
                let body = compress_tar(
                    AssetKind::TarGz,
                    &archive_with_first_path(NORMAL_TAR, &outside),
                );
                let client = StaticResponseClient { body };
                let result = download_server_binary(
                    &client,
                    "https://example.com/release.tar.gz",
                    None,
                    &destination,
                    AssetKind::TarGz,
                )
                .await;
                match result {
                    Ok(()) => assert!(destination.is_dir()),
                    Err(error) => {
                        assert!(format!("{error:#}").contains("extracting"));
                        assert!(!destination.exists());
                    }
                }
                assert!(!outside.exists());
            }
        });
    }

    #[test]
    fn downloads_raw_binary_into_destination_dir() {
        futures::executor::block_on(async {
            let temp_dir = tempfile::tempdir().unwrap();
            let destination_path = temp_dir.path().join("v_1");
            let contents = b"#!/bin/sh\necho hello\n".to_vec();
            let expected_sha_256 = sha256_hex(Sha256::new_with_prefix(&contents));
            let client = StaticResponseClient { body: contents };

            download_server_raw_binary(
                &client,
                "https://example.com/agent-binary",
                Some(&expected_sha_256),
                &destination_path,
                "agent-binary",
            )
            .await
            .unwrap();

            let binary_path = destination_path.join("agent-binary");
            assert_eq!(
                std::fs::read(&binary_path).unwrap(),
                b"#!/bin/sh\necho hello\n"
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&binary_path)
                    .unwrap()
                    .permissions()
                    .mode();
                assert_eq!(mode & 0o111, 0o111, "binary should be executable");
            }
        });
    }

    #[test]
    fn raw_binary_digest_mismatch_cleans_up_staging() {
        futures::executor::block_on(async {
            let temp_dir = tempfile::tempdir().unwrap();
            let destination_path = temp_dir.path().join("v_1");
            let client = StaticResponseClient {
                body: b"some binary".to_vec(),
            };

            let error = download_server_raw_binary(
                &client,
                "https://example.com/agent-binary",
                Some("0000000000000000000000000000000000000000000000000000000000000000"),
                &destination_path,
                "agent-binary",
            )
            .await
            .unwrap_err();

            assert!(error.to_string().contains("SHA-256 mismatch"));
            assert!(!destination_path.exists());
            let leftover_entries = std::fs::read_dir(temp_dir.path()).unwrap().count();
            assert_eq!(leftover_entries, 0, "staging directory should be removed");
        });
    }
}
