use adapter_internals::InternalsError;
use adapter_rtcstats::RtcStatsError;
use flate2::read::MultiGzDecoder;
use rtc_domain::RtcSession;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use thiserror::Error;

pub const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_EXPANDED_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum DumpError {
    #[error("failed to inspect dump file: {0}")]
    Metadata(#[source] io::Error),
    #[error("failed to open dump file: {0}")]
    Open(#[source] io::Error),
    #[error("dump file is empty")]
    Empty,
    #[error("dump file is too large ({actual} bytes, maximum {maximum} bytes)")]
    InputTooLarge { actual: u64, maximum: u64 },
    #[error("expanded dump exceeded {maximum} bytes")]
    ExpandedTooLarge { maximum: u64 },
    #[error("unrecognized dump format")]
    UnknownFormat,
    #[error(transparent)]
    RtcStats(RtcStatsError),
    #[error(transparent)]
    Internals(InternalsError),
}

impl DumpError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Metadata(_) | Self::Open(_) => "dump.file-read-failed",
            Self::Empty => "dump.empty",
            Self::InputTooLarge { .. } => "dump.input-too-large",
            Self::ExpandedTooLarge { .. } => "dump.expanded-too-large",
            Self::UnknownFormat => "dump.unknown-format",
            Self::RtcStats(RtcStatsError::InvalidHeader) => "rtcstats.invalid-header",
            Self::RtcStats(
                RtcStatsError::InvalidMetadata(_) | RtcStatsError::InvalidMetadataType,
            ) => "rtcstats.invalid-metadata",
            Self::RtcStats(RtcStatsError::InvalidJson { .. }) => "rtcstats.invalid-json",
            Self::RtcStats(_) => "rtcstats.invalid-event",
            Self::Internals(InternalsError::InvalidJson(_)) => "internals.invalid-json",
            Self::Internals(InternalsError::InvalidRoot | InternalsError::InvalidStructure) => {
                "internals.invalid-structure"
            }
        }
    }
}

pub fn parse_path(path: &Path) -> Result<RtcSession, DumpError> {
    let metadata = path.metadata().map_err(DumpError::Metadata)?;
    if metadata.len() == 0 {
        return Err(DumpError::Empty);
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(DumpError::InputTooLarge {
            actual: metadata.len(),
            maximum: MAX_INPUT_BYTES,
        });
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dump")
        .to_string();
    let mut file = File::open(path).map_err(DumpError::Open)?;
    let mut magic = [0_u8; 2];
    let magic_len = file.read(&mut magic).map_err(DumpError::Open)?;
    drop(file);
    let compressed = magic_len == 2 && magic == [0x1f, 0x8b];

    let file = File::open(path).map_err(DumpError::Open)?;
    let input: Box<dyn Read + Send> = if compressed {
        Box::new(MultiGzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let limited = LimitedReader::new(input, MAX_EXPANDED_BYTES);
    let reader = BufReader::new(limited);
    dispatch(reader, file_name, metadata.len(), compressed)
}

fn dispatch<R: BufRead>(
    mut reader: R,
    file_name: String,
    file_size: u64,
    compressed: bool,
) -> Result<RtcSession, DumpError> {
    let prefix = reader.fill_buf().map_err(map_limited_error)?;
    if prefix.is_empty() {
        return Err(DumpError::Empty);
    }
    if prefix.starts_with(b"RTCStatsDump\n") || prefix.starts_with(b"RTCStatsDump\r\n") {
        return adapter_rtcstats::parse(reader, file_name, file_size, compressed)
            .map_err(map_rtcstats_error);
    }
    if prefix
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        == Some(b'{')
    {
        return adapter_internals::parse(reader, file_name, file_size, compressed)
            .map_err(map_internals_error);
    }
    Err(DumpError::UnknownFormat)
}

fn map_rtcstats_error(error: RtcStatsError) -> DumpError {
    match error {
        RtcStatsError::Read { source, .. } if source.kind() == io::ErrorKind::FileTooLarge => {
            DumpError::ExpandedTooLarge {
                maximum: MAX_EXPANDED_BYTES,
            }
        }
        other => DumpError::RtcStats(other),
    }
}

fn map_internals_error(error: InternalsError) -> DumpError {
    match error {
        InternalsError::InvalidJson(source)
            if source.io_error_kind() == Some(io::ErrorKind::FileTooLarge) =>
        {
            DumpError::ExpandedTooLarge {
                maximum: MAX_EXPANDED_BYTES,
            }
        }
        other => DumpError::Internals(other),
    }
}

fn map_limited_error(error: io::Error) -> DumpError {
    if error.kind() == io::ErrorKind::FileTooLarge {
        DumpError::ExpandedTooLarge {
            maximum: MAX_EXPANDED_BYTES,
        }
    } else {
        DumpError::Open(error)
    }
}

struct LimitedReader<R> {
    inner: R,
    remaining: u64,
}

impl<R> LimitedReader<R> {
    fn new(inner: R, maximum: u64) -> Self {
        Self {
            inner,
            remaining: maximum,
        }
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut probe = [0_u8; 1];
            return match self.inner.read(&mut probe)? {
                0 => Ok(0),
                _ => Err(io::Error::new(
                    io::ErrorKind::FileTooLarge,
                    "expanded dump size limit exceeded",
                )),
            };
        }
        let allowed =
            usize::try_from(self.remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = self.inner.read(&mut buffer[..allowed])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_path, DumpError, LimitedReader};
    use flate2::{write::GzEncoder, Compression};
    use rtc_domain::DumpFormat;
    use std::fs;
    use std::io::{Cursor, Read, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    struct TempDump(PathBuf);

    impl TempDump {
        fn new(extension: &str, contents: &[u8]) -> Self {
            let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rtc-inspector-{}-{sequence}.{extension}",
                std::process::id()
            ));
            fs::write(&path, contents).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDump {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn workspace_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    #[test]
    fn parses_uncompressed_rtcstats_dump() {
        let input = b"RTCStatsDump\n{\"fileFormat\":3}\n[39,\"pc\",{},1]\n";
        let dump = TempDump::new("jsonl", input);
        let session = parse_path(&dump.0).unwrap();
        assert_eq!(session.source.format, DumpFormat::RtcStats);
        assert_eq!(session.peer_connections.len(), 1);
        assert_eq!(session.peer_connections[0].events.len(), 1);
    }

    #[test]
    fn parses_gzipped_internals_dump() {
        let input = br#"{
            "timestamp": 1000,
            "PeerConnections": {
                "pc": {
                    "updateLog": [
                        {"type": "iceconnectionstatechange", "value": "connected", "timestamp": 1100}
                    ]
                }
            }
        }"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        let dump = TempDump::new("json.gz", &encoder.finish().unwrap());
        let session = parse_path(&dump.0).unwrap();
        assert_eq!(session.source.format, DumpFormat::WebRtcInternals);
        assert!(session.source.compressed);
        assert!(!session.client_events.is_empty());
    }

    #[test]
    fn rejects_unknown_format() {
        let result = parse_path(&workspace_path("README.md"));
        assert!(matches!(result, Err(DumpError::UnknownFormat)));
    }

    #[test]
    fn parses_gzipped_rtcstats_dump() {
        let input = b"RTCStatsDump\n{\"fileFormat\":3}\n[39,\"pc\",{},1]\n";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        let dump = TempDump::new("jsonl.gz", &encoder.finish().unwrap());
        let session = parse_path(&dump.0).unwrap();
        assert_eq!(session.source.format, DumpFormat::RtcStats);
        assert!(session.source.compressed);
    }

    #[test]
    fn rejects_damaged_gzip_as_a_read_error() {
        let dump = TempDump::new("gz", &[0x1f, 0x8b, 0x08, 0x00, 0x01]);
        let error = parse_path(&dump.0).unwrap_err();
        assert_eq!(error.code(), "dump.file-read-failed");
    }

    #[test]
    fn limits_expanded_input() {
        let mut reader = LimitedReader::new(Cursor::new(b"1234"), 3);
        let mut output = Vec::new();
        let error = reader.read_to_end(&mut output).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::FileTooLarge);
        assert_eq!(output, b"123");
    }
}
