//! Files (M6a, Step 1b-5, D91-D98): `open`'s modes and an open file's
//! state; for text, UTF-8 decoding with CPython's errors and universal
//! newlines on read, the platform's line separator on write. Safe and
//! free of the C ABI, so the compiler includes this file too (its
//! `lib.rs`): one source for both backends and for the modes inference
//! accepts.

use std::borrow::Cow;
use std::fs;
use std::io::{self, BufWriter, Read, Write};

/// A literal `mode` of `open` (D91).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    R,
    Rt,
    W,
    Wt,
    A,
    At,
    Rb,
    Wb,
    Ab,
}

impl Mode {
    pub const ALL: [Mode; 9] = [
        Mode::R,
        Mode::Rt,
        Mode::W,
        Mode::Wt,
        Mode::A,
        Mode::At,
        Mode::Rb,
        Mode::Wb,
        Mode::Ab,
    ];

    /// The mode as a program writes it.
    pub fn literal(self) -> &'static str {
        match self {
            Mode::R => "r",
            Mode::Rt => "rt",
            Mode::W => "w",
            Mode::Wt => "wt",
            Mode::A => "a",
            Mode::At => "at",
            Mode::Rb => "rb",
            Mode::Wb => "wb",
            Mode::Ab => "ab",
        }
    }

    /// The mode a literal spells, if supported.
    pub fn parse(literal: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.literal() == literal)
    }

    /// The kind of file it opens.
    pub fn kind(self) -> Kind {
        match self {
            Mode::R | Mode::Rt => Kind::TextReader,
            Mode::W | Mode::Wt | Mode::A | Mode::At => Kind::TextWriter,
            Mode::Rb => Kind::BytesReader,
            Mode::Wb | Mode::Ab => Kind::BytesWriter,
        }
    }

    /// Opens `path` in this mode. A directory fails as CPython's does,
    /// though Unix opens one for reading.
    fn open(self, path: &str) -> io::Result<fs::File> {
        let file = match self {
            Mode::R | Mode::Rt | Mode::Rb => fs::File::open(path)?,
            Mode::W | Mode::Wt | Mode::Wb => fs::File::create(path)?,
            Mode::A | Mode::At | Mode::Ab => fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)?,
        };
        if file.metadata()?.is_dir() {
            return Err(is_a_directory());
        }
        Ok(file)
    }
}

/// What a file of a mode is, which the methods it has depend on (D92).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    TextReader,
    TextWriter,
    BytesReader,
    BytesWriter,
}

impl Kind {
    /// Whether the file holds text, not bytes.
    pub fn is_text(self) -> bool {
        matches!(self, Kind::TextReader | Kind::TextWriter)
    }

    /// Whether the file is written, not read.
    pub fn writes(self) -> bool {
        matches!(self, Kind::TextWriter | Kind::BytesWriter)
    }

    /// CPython's class of the file, in its module `_io`.
    pub fn qualified(self) -> &'static str {
        match self {
            Kind::TextReader | Kind::TextWriter => "_io.TextIOWrapper",
            Kind::BytesReader => "_io.BufferedReader",
            Kind::BytesWriter => "_io.BufferedWriter",
        }
    }

    /// CPython's class of the file, without its module.
    pub fn class(self) -> &'static str {
        let qualified = self.qualified();
        qualified.strip_prefix("_io.").unwrap_or(qualified)
    }
}

#[cfg(unix)]
fn is_a_directory() -> io::Error {
    io::Error::from_raw_os_error(21) // EISDIR on Linux and macOS.
}

#[cfg(not(unix))]
fn is_a_directory() -> io::Error {
    io::Error::from(io::ErrorKind::IsADirectory)
}

/// Why an operation on a file failed.
#[derive(Debug)]
pub enum FileError {
    /// An `OSError`.
    Os(io::Error),
    /// Invalid UTF-8: CPython's `UnicodeDecodeError` message.
    Decode(String),
    /// The file is closed: CPython's `ValueError` message.
    Closed(&'static str),
}

impl From<ReadError> for FileError {
    fn from(e: ReadError) -> Self {
        match e {
            ReadError::Io(e) => FileError::Os(e),
            ReadError::Decode(m) => FileError::Decode(m),
        }
    }
}

impl From<io::Error> for FileError {
    fn from(e: io::Error) -> Self {
        FileError::Os(e)
    }
}

/// What `read()` gives: text, or bytes for a binary file.
#[derive(Debug, PartialEq, Eq)]
pub enum Data {
    Text(String),
    Bytes(Vec<u8>),
}

/// An open file, or one closed: its name, mode and encoding as `open`
/// got them, for its `repr`.
#[derive(Debug)]
pub struct File {
    name: Box<str>,
    mode: Mode,
    encoding: Box<str>,
    io: Option<Io>,
}

/// How an open file is read or written. A writer's buffer is flushed on
/// drop, any error dropped (D98).
#[derive(Debug)]
enum Io {
    Read(fs::File, Decoder),
    Write(BufWriter<fs::File>),
}

/// CPython's message for an operation on a closed file of `mode`.
fn closed(kind: Kind, op: &str) -> FileError {
    let msg = match (kind.is_text(), kind.writes(), op) {
        (false, false, "read") => "read of closed file",
        (false, true, "write") => "write to closed file",
        (false, true, "flush") => "flush of closed file",
        _ => "I/O operation on closed file.",
    };
    FileError::Closed(msg)
}

impl File {
    /// `open(name, mode, encoding=encoding)`.
    pub fn open(name: &str, mode: Mode, encoding: &str) -> io::Result<File> {
        let file = mode.open(name)?;
        let io = match mode.kind().writes() {
            true => Io::Write(BufWriter::new(file)),
            false => Io::Read(file, Decoder::default()),
        };
        Ok(File {
            name: name.into(),
            mode,
            encoding: encoding.into(),
            io: Some(io),
        })
    }

    /// The kind of file it is.
    pub fn kind(&self) -> Kind {
        self.mode.kind()
    }

    /// Python's `repr`, the name shown by `quote`, its `repr` as a
    /// `str`.
    pub fn repr(&self, quote: impl Fn(&str) -> String) -> String {
        let (class, quoted_name) = (self.kind().qualified(), quote(&self.name));
        match self.kind().is_text() {
            true => format!(
                "<{class} name={quoted_name} mode='{}' encoding='{}'>",
                self.mode.literal(),
                self.encoding
            ),
            false => format!("<{class} name={quoted_name}>"),
        }
    }

    /// The reader `op` reads with, or why there is none.
    fn reader(&mut self, op: &str) -> Result<(&mut fs::File, &mut Decoder), FileError> {
        let kind = self.kind();
        match &mut self.io {
            Some(Io::Read(file, decoder)) => Ok((file, decoder)),
            _ => Err(closed(kind, op)),
        }
    }

    /// The writer `op` writes with, or why there is none.
    fn writer(&mut self, op: &str) -> Result<&mut BufWriter<fs::File>, FileError> {
        let kind = self.kind();
        match &mut self.io {
            Some(Io::Write(w)) => Ok(w),
            _ => Err(closed(kind, op)),
        }
    }

    /// `f.read()`: the rest of the file.
    pub fn read(&mut self) -> Result<Data, FileError> {
        let text = self.kind().is_text();
        let (file, decoder) = self.reader("read")?;
        if text {
            return Ok(Data::Text(decoder.read_all(file)?));
        }
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(Data::Bytes(data))
    }

    /// `f.readline()` of a text file: `""` at the end.
    pub fn readline(&mut self) -> Result<String, FileError> {
        let (file, decoder) = self.reader("readline")?;
        Ok(decoder.readline(file)?)
    }

    /// `f.write(s)` of a text file: how many code points it wrote.
    pub fn write_text(&mut self, s: &str) -> Result<usize, FileError> {
        self.writer("write")?.write_all(encode(s).as_bytes())?;
        Ok(s.chars().count())
    }

    /// `f.write(b)` of a binary file: how many bytes it wrote.
    pub fn write_bytes(&mut self, b: &[u8]) -> Result<usize, FileError> {
        self.writer("write")?.write_all(b)?;
        Ok(b.len())
    }

    /// `f.flush()`: a reader has nothing to flush.
    pub fn flush(&mut self) -> Result<(), FileError> {
        if self.io.is_none() {
            return Err(closed(self.kind(), "flush"));
        }
        match &mut self.io {
            Some(Io::Write(w)) => Ok(w.flush()?),
            _ => Ok(()),
        }
    }

    /// `f.close()`: flushes a writer, then closes; closing again does
    /// nothing. The file is closed even when the flush fails.
    pub fn close(&mut self) -> io::Result<()> {
        match self.io.take() {
            Some(Io::Write(mut w)) => w.flush(),
            _ => Ok(()),
        }
    }
}

/// How many bytes a line read decodes at a time: CPython's
/// `TextIOWrapper._CHUNK_SIZE`, which positions in its decode errors
/// count within.
pub const CHUNK: usize = 8192;

/// Why a text read failed.
#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    /// Invalid UTF-8: CPython's `UnicodeDecodeError` message.
    Decode(String),
}

impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> Self {
        ReadError::Io(e)
    }
}

/// A text reader's state between reads: bytes that end in the middle of
/// a character, text decoded but not yet returned, and a `"\r"` held
/// back in case a `"\n"` follows.
#[derive(Debug, Default)]
pub struct Decoder {
    pending: Vec<u8>,
    text: String,
    cr: bool,
    eof: bool,
}

impl Decoder {
    /// `f.read()`: the rest of `src`, decoded in one piece, so an error's
    /// position counts from where this read started.
    pub fn read_all(&mut self, src: &mut impl Read) -> Result<String, ReadError> {
        let mut raw = std::mem::take(&mut self.pending);
        src.read_to_end(&mut raw)?;
        self.eof = true;
        let decoded = decode(&raw, true)?;
        self.push(&decoded.0, true);
        Ok(std::mem::take(&mut self.text))
    }

    /// `f.readline()`: up to and including the next `"\n"`, or the rest;
    /// `""` at the end.
    pub fn readline(&mut self, src: &mut impl Read) -> Result<String, ReadError> {
        loop {
            if let Some(at) = self.text.find('\n') {
                return Ok(self.text.drain(..=at).collect());
            }
            if self.eof {
                return Ok(std::mem::take(&mut self.text));
            }
            self.fill(src)?;
        }
    }

    /// Decodes the next chunk of `src`, or ends the input.
    fn fill(&mut self, src: &mut impl Read) -> Result<(), ReadError> {
        let mut raw = std::mem::take(&mut self.pending);
        let start = raw.len();
        raw.resize(start + CHUNK, 0);
        let got = read_full(src, &mut raw[start..])?;
        raw.truncate(start + got);
        let last = got == 0;
        self.eof = last;
        let (text, rest) = decode(&raw, last)?;
        self.pending = raw.split_off(raw.len() - rest);
        self.push(&text, last);
        Ok(())
    }

    /// Appends decoded `s` with `"\r\n"` and `"\r"` read as `"\n"`. A
    /// `"\r"` ending `s` waits for the next piece unless `last`.
    fn push(&mut self, s: &str, last: bool) {
        let mut s = match std::mem::take(&mut self.cr) {
            true => format!("\r{s}"),
            false => s.to_string(),
        };
        if !last && s.ends_with('\r') {
            s.pop();
            self.cr = true;
        }
        self.text
            .push_str(&s.replace("\r\n", "\n").replace('\r', "\n"));
    }
}

/// Reads until `buf` is full or `src` ends: how many bytes it read.
fn read_full(src: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        match src.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(got)
}

/// Decodes `raw` as UTF-8: the text and how many bytes at its end start a
/// character not yet complete (none when `last`). Invalid input is
/// CPython's `UnicodeDecodeError` message.
pub fn decode(raw: &[u8], last: bool) -> Result<(String, usize), ReadError> {
    let err = match std::str::from_utf8(raw) {
        Ok(s) => return Ok((s.to_string(), 0)),
        Err(e) => e,
    };
    let at = err.valid_up_to();
    let valid = String::from_utf8_lossy(&raw[..at]).into_owned();
    match err.error_len() {
        None if !last => Ok((valid, raw.len() - at)),
        None => Err(decode_error(
            raw,
            at,
            raw.len() - at,
            "unexpected end of data",
        )),
        Some(n) => Err(decode_error(raw, at, n, reason(raw[at]))),
    }
}

/// Why the sequence starting with `lead` is invalid, as CPython says.
fn reason(lead: u8) -> &'static str {
    match lead {
        0x80..=0xc1 | 0xf5..=0xff => "invalid start byte",
        _ => "invalid continuation byte",
    }
}

/// CPython's message for the `n` bad bytes of `raw` at `at`.
fn decode_error(raw: &[u8], at: usize, n: usize, why: &str) -> ReadError {
    let msg = match n {
        1 => format!(
            "'utf-8' codec can't decode byte 0x{:02x} in position {at}: {why}",
            raw[at]
        ),
        _ => format!(
            "'utf-8' codec can't decode bytes in position {at}-{}: {why}",
            at + n - 1
        ),
    };
    ReadError::Decode(msg)
}

/// What writing `s` to a text file writes: `"\n"` as the platform's line
/// separator, as CPython does.
pub fn encode(s: &str) -> Cow<'_, str> {
    match cfg!(windows) {
        true => Cow::Owned(s.replace('\n', "\r\n")),
        false => Cow::Borrowed(s),
    }
}

/// Whether `name`, an `encoding=` literal, names UTF-8 (D95): lowercase,
/// with `-` read as `_`, `utf_8` or `utf8`.
pub fn is_utf8(name: &str) -> bool {
    let norm = name.to_ascii_lowercase().replace('-', "_");
    norm == "utf_8" || norm == "utf8"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(data: &[u8]) -> Vec<String> {
        let (mut d, mut src) = (Decoder::default(), data);
        let mut out = Vec::new();
        loop {
            match d.readline(&mut src) {
                Ok(l) if l.is_empty() => return out,
                Ok(l) => out.push(l),
                Err(e) => panic!("{e:?}"),
            }
        }
    }

    fn message(r: Result<impl std::fmt::Debug, ReadError>) -> String {
        match r {
            Err(ReadError::Decode(m)) => m,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn universal_newlines_across_chunks() {
        assert_eq!(lines(b"a\r\nb\rc\nd"), ["a\n", "b\n", "c\n", "d"]);
        let mut split = vec![b'x'; CHUNK - 1];
        split.extend_from_slice(b"\r\ny");
        let got = lines(&split);
        assert_eq!(got.len(), 2);
        assert!(got[0].ends_with("x\n"));
        assert_eq!(got[1], "y");
        assert_eq!(lines(b"end\r"), ["end\n"]);
    }

    #[test]
    fn a_character_split_across_chunks_decodes_whole() {
        let mut data = vec![b'x'; CHUNK - 1];
        data.extend_from_slice("é\n".as_bytes());
        let got = lines(&data);
        assert!(got[0].ends_with("xé\n"), "{got:?}");
    }

    #[test]
    fn read_returns_what_readline_left() {
        let (mut d, mut src) = (Decoder::default(), &b"one\ntwo\r\nthree"[..]);
        assert_eq!(d.readline(&mut src).ok().as_deref(), Some("one\n"));
        assert_eq!(d.read_all(&mut src).ok().as_deref(), Some("two\nthree"));
        assert_eq!(d.readline(&mut src).ok().as_deref(), Some(""));
    }

    #[test]
    fn decode_errors_say_what_cpython_says() {
        let msg = |b: &[u8]| message(decode(b, true));
        assert_eq!(
            msg(b"ab\xff"),
            "'utf-8' codec can't decode byte 0xff in position 2: invalid start byte"
        );
        assert_eq!(
            msg(b"\xe2\x28\xa1"),
            "'utf-8' codec can't decode byte 0xe2 in position 0: invalid continuation byte"
        );
        assert_eq!(
            msg(b"\xf0\x9f\x98("),
            "'utf-8' codec can't decode bytes in position 0-2: invalid continuation byte"
        );
        assert_eq!(
            msg(b"a\xe2\x82"),
            "'utf-8' codec can't decode bytes in position 1-2: unexpected end of data"
        );
        assert_eq!(
            msg(b"\xe2"),
            "'utf-8' codec can't decode byte 0xe2 in position 0: unexpected end of data"
        );
        assert!(matches!(decode(b"a\xe2\x82", false), Ok((ref s, 2)) if s == "a"));
        assert!(matches!(decode("\u{feff}x".as_bytes(), true), Ok((ref s, 0)) if s == "\u{feff}x"));
    }

    fn temp(tag: &str) -> String {
        let name = format!("pythrust-textio-{tag}-{}", std::process::id());
        std::env::temp_dir()
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    fn opened(path: &str, mode: Mode) -> File {
        match File::open(path, mode, "utf-8") {
            Ok(f) => f,
            Err(e) => panic!("{path}: {e}"),
        }
    }

    #[test]
    fn a_text_file_writes_then_reads_back() {
        let path = temp("text");
        let mut w = opened(&path, Mode::W);
        assert_eq!(w.write_text("héllo\nwörld").ok(), Some(11));
        assert_eq!(
            w.repr(|_| "'p'".into()),
            "<_io.TextIOWrapper name='p' mode='w' encoding='utf-8'>"
        );
        assert!(w.close().is_ok());
        let mut a = opened(&path, Mode::A);
        assert_eq!(a.write_text("!\n").ok(), Some(2));
        drop(a);
        let mut r = opened(&path, Mode::R);
        assert_eq!(r.readline().ok().as_deref(), Some("héllo\n"));
        assert_eq!(r.read().ok(), Some(Data::Text("wörld!\n".into())));
        assert_eq!(r.readline().ok().as_deref(), Some(""));
        let mut rb = opened(&path, Mode::Rb);
        let bytes = encode("héllo\nwörld!\n").as_bytes().to_vec();
        assert_eq!(rb.read().ok(), Some(Data::Bytes(bytes)));
        assert_eq!(rb.repr(|_| "'p'".into()), "<_io.BufferedReader name='p'>");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_closed_file_says_what_cpython_says() {
        let path = temp("closed");
        fn message<T: std::fmt::Debug>(e: Result<T, FileError>) -> &'static str {
            match e {
                Err(FileError::Closed(m)) => m,
                other => panic!("{other:?}"),
            }
        }
        let mut wb = opened(&path, Mode::Wb);
        assert_eq!(wb.write_bytes(b"ab").ok(), Some(2));
        assert!(wb.close().is_ok());
        assert!(wb.close().is_ok());
        assert_eq!(message(wb.write_bytes(b"")), "write to closed file");
        assert_eq!(message(wb.flush()), "flush of closed file");
        let mut rb = opened(&path, Mode::Rb);
        assert!(rb.close().is_ok());
        assert_eq!(message(rb.read()), "read of closed file");
        assert_eq!(message(rb.flush()), "I/O operation on closed file.");
        let mut t = opened(&path, Mode::Rt);
        assert!(t.flush().is_ok());
        assert!(t.close().is_ok());
        for e in [t.read().map(|_| ()), t.readline().map(|_| ()), t.flush()] {
            assert_eq!(message(e), "I/O operation on closed file.");
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn modes_parse_their_literals_only() {
        for m in Mode::ALL {
            assert_eq!(Mode::parse(m.literal()), Some(m));
        }
        for bad in ["br", "x", "r+", "", "rw"] {
            assert_eq!(Mode::parse(bad), None, "{bad}");
        }
        assert_eq!(Mode::Ab.kind().class(), "BufferedWriter");
        assert_eq!(Mode::At.kind(), Kind::TextWriter);
        assert!(Kind::TextWriter.writes() && Kind::TextWriter.is_text());
    }

    #[cfg(unix)]
    #[test]
    fn opening_a_directory_fails_in_every_mode() {
        let dir = std::env::temp_dir().to_string_lossy().into_owned();
        for m in [Mode::R, Mode::W, Mode::Ab] {
            let kind = File::open(&dir, m, "utf-8")
                .map(|_| ())
                .map_err(|e| e.kind());
            assert_eq!(kind, Err(io::ErrorKind::IsADirectory), "{m:?}");
        }
    }

    #[test]
    fn utf8_spellings() {
        for ok in ["utf-8", "UTF-8", "utf8", "UTF8", "utf_8", "Utf-8"] {
            assert!(is_utf8(ok), "{ok}");
        }
        for bad in ["u8", "cp65001", "latin-1", ""] {
            assert!(!is_utf8(bad), "{bad}");
        }
    }
}
