use super::KvsEngine;
use crate::error::{KvsError, Result};

use std::cell::RefCell;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use clap::crate_version;
use dashmap::DashMap;
use tracing::{debug, info, error, instrument};
use tracing::field::debug;

// the size of stale data, in bytes, that will trigger a log compaction
const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// A multi-threaded, key-value storage engine implementation.
///
/// Keys and values are persisted across a series of "command logs" located on the local file system.
/// Each log will have a filename that begins with am integer and ends with the suffix ".log"
/// i.e. "1.log", "2.log" etc...
///
/// Once the size of "stale" data in the command logs hits the COMPACTION_THRESHOLD, the files
/// will be compacted into a new log file and unused log files will be deleted.
///
/// # Examples
/// ```rust
/// use kvs::{KvsEngine, KvStore};
/// use std::path::Path;
///
/// // create and open a new KvStore, using the current directory to persist key/value data
/// let kvs = KvStore::open(Path::new("."));
///
/// // set a key and value in the store
/// kvs.set("myKey".to_string(), "myValue".to_string());
///
/// // retrieve the value of "myKey"
/// kvs.get("myKey".to_string());  // Ok(Some("myValue"))
///
/// // remove a key and value from the store
/// kvs.remove("myKey".to_string()); // Ok(())
///
/// // remove a key that doesn't exist
/// kvs.remove("fakeKey".to_string()); // Err(KvsError::KeyNotFound)
/// ```
#[derive(Debug, Clone)]
pub struct KvStore {
    // the directory containing the command log files
    //working_dir: Arc<PathBuf>,

    // every KvStore gets its own single-threaded reader
    reader: KvsReader,

    // writer of the current command log.
    writer: Arc<Mutex<KvsWriter>>,

    // maps a key to the position of its value within a log file
    index: Arc<DashMap<String, CommandPos>>,
}

impl KvStore {

    /// creates a [`KvStore`] using the given `working_dir` Path as the directory for the command
    /// logs. If the `working_dir` does not exist it will be created.
    ///
    /// # Errors
    /// [`KvsError::Io`] is returned if the working_dir could not be created
    #[instrument]
    pub fn open(working_dir: &Path) -> Result<KvStore> {
        info!("opening KVS engine version {}", crate_version!());
        fs::create_dir_all(working_dir)?;
        debug!("working_dir path= {:?}", working_dir.canonicalize().unwrap().to_str());
        let path = Arc::new(working_dir.to_path_buf());

        // get all log gen numbers in the working dir
        let log_gens = get_log_gens(&path)?.unwrap_or_default();
        debug!(?log_gens);

        let mut readers = BTreeMap::new();
        let index = Arc::new(DashMap::new());
        let mut uncompacted = 0_u64;

        // build buffered readers for all log files in the working_dir
        for gen in &log_gens {
            let mut reader =
                BufReaderWithPos::new(File::open(build_log_path(&path, *gen))?)?;
            // load data from the reader into the index
            uncompacted += load(*gen, &mut reader, &index)?;
            readers.insert(*gen, reader);
        }
        debug!(?uncompacted);

        // determine the largest generation number
        let current_log_gen = log_gens.last().unwrap_or(&0) + 1;
        debug!(?current_log_gen);

        // build a KvsReader for all the command log files currently in use
        let reader = KvsReader {
            path: path.clone(),
            readers: RefCell::new(readers),
            latest_compaction_gen: Arc::new(AtomicU64::new(0)),
        };

        // build a new log file where new commands will be written to
        let buf_writer = new_log_file(&path, current_log_gen)?;
        let writer = KvsWriter {
            reader: reader.clone(),
            writer: buf_writer,
            uncompacted,
            current_gen: current_log_gen,
            path: path.clone(),
            index: index.clone(),
        };

        Ok(KvStore {
            //working_dir: path.clone(),
            index: index.clone(),
            reader,
            writer: Arc::new(Mutex::new(writer)),
        })
    }
}

impl KvsEngine for KvStore {

    fn set(&self, key: String, value: String) -> Result<()> {
        self.writer.lock().unwrap().set(key, value)
    }

    #[instrument]
    fn get(&self, key: String) -> Result<Option<String>> {
        // check for existence of key in index
        if let Some(command) = self.index.get(&key) {
            // get a reader based on the command generation
            if let Command::Set { value, .. } = self.reader.read_command(*command.value())? {
                Ok(Some(value))
            } else {
                error!("could not get command for key: {} command: {:?}", &key, &command.value());
                Err(KvsError::InvalidCommand(format!("invalid command in logs for key: {}", &key)))
            }
        } else {
            Ok(None)
        }
    }

    fn remove(&self, key: String) -> Result<()> {
        self.writer.lock().unwrap().remove(key)
    }
}

/// `KvsReader` maintains a map of readers to all command logs currently in use.
///
/// Every `KvStore` instance has its own `KvsReader` and every `KvsReader`
/// opens the same files separately; so a `KvsReader` can read concurrently through
/// multiple `KvStore`s in different threads.
#[derive(Debug)]
struct KvsReader {
    path: Arc<PathBuf>,

    readers: RefCell<BTreeMap<u64, BufReaderWithPos<File>>>,

    // generation of the latest compaction file
    latest_compaction_gen: Arc<AtomicU64>,
}

impl KvsReader {

    /// Removes handles to files that are no longer needed.
    ///
    /// Files are no longer needed when their generation number is less than the
    /// `latest_compaction_gen`. Files will become "stale" after a compaction
    /// finishes, so there is no point keeping them around, the latest compaction file
    /// will have the sum of all generational files before it
    fn remove_stale_handles(&self) {
        let mut readers = self.readers.borrow_mut();
        while !readers.is_empty() {
            let first_gen = *readers.keys().next().unwrap();
            if self.latest_compaction_gen.load(Ordering::SeqCst) <= first_gen {
                break;
            }
            readers.remove(&first_gen);
        }
    }

    /// Read the log file at the given `CommandPos`.
    fn read_and<F, R>(&self, cmd_pos: CommandPos, f: F) -> Result<R>
        where
            F: FnOnce(io::Take<&mut BufReaderWithPos<File>>) -> Result<R>,
    {
        self.remove_stale_handles();

        let mut readers = self.readers.borrow_mut();

        // Open the file if we haven't opened it in this `KvStoreReader`.
        // We don't use entry API here because we want the errors to be propagated.
        if let Entry::Vacant(e) = readers.entry(cmd_pos.gen) {
            let reader = BufReaderWithPos::new(File::open(build_log_path(&self.path, cmd_pos.gen))?)?;
            e.insert(reader);
        }

        let reader = readers.get_mut(&cmd_pos.gen).unwrap();
        reader.seek(SeekFrom::Start(cmd_pos.pos))?;
        let cmd_reader = reader.take(cmd_pos.len);
        f(cmd_reader)
    }

    /// Read the log file starting at the given `CommandPos` and deserialize it into `Command`.
    fn read_command(&self, cmd_pos: CommandPos) -> Result<Command> {
        self.read_and(cmd_pos, |cmd_reader| {
            Ok(serde_json::from_reader(cmd_reader)?)
        })
    }
}

impl Clone for KvsReader {
    fn clone(&self) -> KvsReader {
        KvsReader {
            path: Arc::clone(&self.path),
            latest_compaction_gen: Arc::clone(&self.latest_compaction_gen),
            // every KvsReader will have their own map of readers
            readers: RefCell::new(BTreeMap::new()),
        }
    }
}

#[derive(Debug)]
struct KvsWriter {
    reader: KvsReader,
    writer: BufWriterWithPos<File>,

    // the current log generation number
    current_gen: u64,

    // the number of bytes representing "stale" commands that could be
    // deleted during a compaction
    uncompacted: u64,

    // the path to the directory containing the kvs logs files
    path: Arc<PathBuf>,

    // a handle to the in-memory index
    index: Arc<DashMap<String, CommandPos>>,
}

impl KvsWriter {

    /// sets the given `key` and `value` into the `index` and also writes them into
    /// the log file
    #[instrument]
    fn set(&mut self, key: String, value: String) -> Result<()> {
        // create a Set command variant
        let cmd = Command::Set { key, value };
        // set pos to the current position of the writer which is usually at the end of the log
        let pos = self.writer.pos;
        // serialize the command into the log using serde and flush the writer
        serde_json::to_writer(&mut self.writer, &cmd)?;
        self.writer.flush()?;

        if let Command::Set { key, .. } = cmd {
            // check if the key currently exists in the index, if so, increment
            // uncompacted with the old.len, as that data is now stale and will be overriden with new key
            if let Some(old_cmd) = self.index.get(&key) {
                self.uncompacted += old_cmd.value().len;
            }
            // insert the key along with its CommandPos data
            self.index.insert(key, (self.current_gen, pos..self.writer.pos).into());
        }

        // run a log compaction if needed
        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    /// remove the given `key` from the index
    #[instrument]
    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::Remove { key };
            let pos = self.writer.pos;
            // serialze the remove command into the log and flush
            serde_json::to_writer(&mut self.writer, &cmd)?;
            self.writer.flush()?;

            if let Command::Remove { key } = cmd {
                let (_key, old_cmd) = self.index.remove(&key).expect("key not found");
                // update uncompacted with the removed length
                self.uncompacted += old_cmd.len;
                // the "remove" command itself can be deleted in the next compaction
                // so we add its length to `uncompacted`
                self.uncompacted += self.writer.pos - pos;
            }

            // run a compaction if needed
            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }
            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }

    /// Clears stale entries in the log.
    #[instrument]
    fn compact(&mut self) -> Result<()> {
        // increase current gen by 2. current_gen + 1 is for the compaction file
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;
        self.writer = new_log_file(&self.path, self.current_gen)?;
        debug!("compaction started, compaction_gen={}, current_gen={}", &compaction_gen, &self.current_gen);

        let mut compaction_writer = new_log_file(&self.path, compaction_gen)?;

        let mut new_pos = 0; // pos in the new log file
        for mut entry in self.index.iter_mut() {
            let len = self.reader.read_and(*entry.value(), |mut entry_reader| {
                Ok(io::copy(&mut entry_reader, &mut compaction_writer)?)
            })?;
            *entry.value_mut() = (compaction_gen, new_pos..new_pos + len).into();
            new_pos += len;
        }
        compaction_writer.flush()?;

        self.reader
            .latest_compaction_gen
            .store(compaction_gen, Ordering::SeqCst);
        self.reader.remove_stale_handles();

        // remove stale log files
        // Note that actually these files are not deleted immediately because `KvStoreReader`s
        // still keep open file handles. When `KvStoreReader` is used next time, it will clear
        // its stale file handles. On Unix, the files will be deleted after all the handles
        // are closed. On Windows, the deletions below will fail and stale files are expected
        // to be deleted in the next compaction.

        let stale_gens = get_log_gens(&self.path)?.unwrap_or_default();
        stale_gens
            .iter()
            .filter(|&&gen| gen < compaction_gen)
            .for_each(|stale_gen| {
                let file_path = build_log_path(&self.path, *stale_gen);
                debug!("{:?} marked as stale", &file_path);
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("{:?} cannot be deleted: {}", file_path, e);
                }
            });
        self.uncompacted = 0;
        debug("compaction finished");
        Ok(())
    }
}

/// loads the commands from the given reader into the store's `index`.
/// Returns the amount of bytes that could be compacted.
/// `gen` is the generation number of the log file being read by `reader`
///
/// # Errors
/// IO Errors will be returned if any log file could not be opened/read
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &DashMap<String, CommandPos>,
) -> Result<u64> {
    let mut pos = reader.seek(SeekFrom::Start(0))?;
    let mut uncompacted = 0_u64;
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

    while let Some(command) = stream.next() {
        let length = stream.byte_offset() as u64 - pos; // length of the command
        match command? {
            Command::Set { key, .. } => {
                if let Some(old_command) =
                index.insert(key, CommandPos::new(gen, pos, length))
                {
                    uncompacted += old_command.len;
                }
            }
            Command::Remove { key } => {
                if let Some((_key, old_command)) = index.remove(&key) {
                    uncompacted += old_command.len;
                }
                // this "remove" command itself can be deleted in the next compaction
                uncompacted += length;
            }
        }
        pos = stream.byte_offset() as u64;
    }

    Ok(uncompacted)
}

/// Constructs a log file path using the `gen` number as the file stem and the appending the
/// suffix **.log** to it. The log file name is then joined to the given `dir` path
fn build_log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// Creates and joins a new log file with the given `gen` number to the given `path`.
/// Returns a new [`BufWriterWithPos`] to the newly created log file.
fn new_log_file(path: &Path, gen: u64) -> Result<BufWriterWithPos<File>> {
    let path = build_log_path(path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?,
    )?;
    Ok(writer)
}

/// These are the command types that will be recorded in the command log(s)
/// NOTE that "GET" commands are not stored in the logs
#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

/// Position data for commands that will be written to a log
#[derive(Debug, Copy, Clone)]
struct CommandPos {
    // the log generation number that the command is stored in
    gen: u64,
    // start position of the command within a log, i.e. the byte offset from the start of the log
    pos: u64,
    // the total length of the command data in bytes
    len: u64,
}

impl CommandPos {
    /// builder method to construct a new `CommandPos`
    fn new(gen: u64, pos: u64, len: u64) -> Self {
        CommandPos { gen, pos, len }
    }
}

impl From<(u64, Range<u64>)> for CommandPos {
    /// Builds a [`CommandPos`] from a tuple of `(generation-number, pos_start..pos_end)`
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

/// Searches for kvs ".log" files within the given `dir`.
/// Returns the generation numbers of all ".log" files that were found, sorted in ascending order.
///
/// This function expects the log files will end with a `.log` suffix and that the
/// file names, (i.e. the file stems), will be valid integer strings.
///
/// # Errors
/// returns an IO Error if the given `dir` and/or log files in that dir could not be read,
/// or if a file stem could not be found for a .log file
/// or if a file stem could not be converted to an integer
fn get_log_gens(dir: &Path) -> Result<Option<Vec<u64>>> {
    let mut logs: Vec<u64> = vec![];

    for entry in (fs::read_dir(dir)?).flatten() {
        if entry.file_type()?.is_file()
            && entry
            .path()
            .extension()
            .map_or(false, |ext| ext.to_str() == Some("log"))
        {
            // get the file stem, convert it into a &str, then try to parse that &str to an integer
            let stem = entry
                .path()
                .file_stem()
                .ok_or_else(|| Error::new(
                    ErrorKind::Other,
                    format!("could not find log file stem for {:?}", &entry.path()),
                ))?
                .to_os_string();
            let gen_str = stem.to_str()
                .map(String::from)
                .ok_or_else(|| Error::new(ErrorKind::Other, format!("could not convert the file stem: {:?} into a str", &stem), ))?;
            let gen = gen_str.parse::<u64>().map_err(|_| {
                KvsError::Parsing(format!(
                    "could not parse the file stem: {} into a u64",
                    &gen_str
                ))
            })?;
            logs.push(gen);
        }
    }
    if !logs.is_empty() {
        logs.sort_unstable();
        Ok(Some(logs))
    } else {
        Ok(None)
    }
}

/// A struct that wraps a [`BufReader`] along with its current seek `pos`ition
#[derive(Debug)]
struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.stream_position()?;
        Ok(BufReaderWithPos {
            reader: BufReader::new(inner),
            pos,
        })
    }
}

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

#[derive(Debug)]
struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> BufWriterWithPos<W> {
    fn new(mut inner: W) -> Result<Self> {
        let pos = inner.stream_position()?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}
