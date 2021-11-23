use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use crate::error::Result;
use anyhow::{anyhow, Context};

// TODO every run of the kvs app generates a new generation log by 1 to hold any newly inserted keys
// need to ensure that keys are i



// size in bytes for triggering a log compaction
const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// the main structure used for working with a KvStore.
/// Data is persisted to "log" files when the "set" command is issued and retrieved from the files
/// when a "get" command is issued.
/// Once the size of "stale" data in the logs hits the COMPACTION THRESHOLD, the log files
/// will be compacted and stale key data will be removed.
#[derive(Debug)]
pub struct KvStore {
    // path to the directory being used to hold log files
    working_dir: PathBuf,

    // the current log generation number being used to store data
    current_log_gen: u64,

    // map log generation number to the file reader
    readers: HashMap<u64, BufReaderWithPos<File>>,

    // writer of the current log.
    writer: BufWriterWithPos<File>,

    // map keys to their location within a log file.
    index: BTreeMap<String, CommandPos>,

    // number of bytes representing "stale" commands that could be
    // deleted during a compaction.
    uncompacted: u64,
}

impl KvStore {

    /// creates a KvStore using the data from the kvs logfile located in the `working_dir`
    /// If the kvs log does not yet exist, it will be created
    pub fn open(working_dir: impl Into<PathBuf>) -> Result<KvStore> {
        let working_dir = working_dir.into();
        fs::create_dir_all(&working_dir)
            .context(format!("could not create directory at {:?}", &working_dir))?;

        // get all log gen numbers in the working dir
        let log_gens = get_log_gens(&working_dir)?.unwrap_or(vec![]);

        let mut readers: HashMap<u64, BufReaderWithPos<File>> = HashMap::new();
        let mut index = BTreeMap::new();
        let mut uncompacted = 0_u64;

        // build buffered readers for all log files
        for gen in &log_gens {
            let mut reader = BufReaderWithPos::new(File::open(build_log_path(&working_dir, *gen))?)?;
            // load data from the reader into the index
            uncompacted += load(*gen, &mut reader, &mut index)?;
            readers.insert(*gen, reader);
        }

        // build a writer into the current gen log
        let current_log_gen = log_gens.last().unwrap_or(&0) + 1;
        let writer = new_log_file(&working_dir, current_log_gen, &mut readers)?;

        Ok(KvStore {
            index,
            readers,
            writer,
            working_dir,
            current_log_gen,
            uncompacted
        })
    }

    /// attempts to retrieve the value associated with `key`.
    /// returns `Some(value)` if the `key` was found, else returns `None`
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    /// ```
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        // check for existence of key in index
        if let Some(CommandPos { gen, pos, len }) = self.index.get(&key) {
            // read the corresponding value from the log
            let reader = self.readers
                .get_mut(gen)
                .expect("reader is missing from readers");

            reader.seek(SeekFrom::Start(*pos))?;
            let cmd_reader = reader.take(*len);
            if let Command::Set { value, .. } = serde_json::from_reader(cmd_reader)? {
                Ok(Some(value))
            } else {
                Err(anyhow!("could not de-serialize command string for key: {}", &key))
            }

        } else {
            Ok(None)
        }
    }

    /// inserts the specified `key` and `value` into this `KvStore`, overriding any existing
    /// key/value entry
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    /// ```
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let cmd = Command::Set { key, value };
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &cmd)?;
        self.writer.flush()?;

        // insert the command into the index
        if let Command::Set { key, .. } = cmd {
            if let Some(old_command) = self
                .index
                .insert(key, (self.current_log_gen, pos..self.writer.pos).into())
            {
                self.uncompacted += old_command.len;
            }
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    /// removes the specified `key` and its associated value from this KvStore
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// ```
    pub fn remove(&mut self, key: String) -> Result<()> {
        if let Some(old_command) = self.index.remove(&key) {
            // creates a value representing the "rm" command, containing its key
            let command = Command::Remove { key };
            // append the serialized command to the log
            serde_json::to_writer(&mut self.writer, &command)?;
            self.writer.flush()?;

            // updated length of uncompacted data
            self.uncompacted += old_command.len;

            Ok(())
        } else {
            Err(anyhow!("Key not found"))
        }

    }

    /// serializes `command` and writes it into the log file
    /// returns a [`CommandPos`] indicating where the command was written at within the log
    // fn write(&mut self, command: &Command) -> Result<CommandPos> {
    //     let serialized = command.serialize()?;
    //     self.writer.write_all(serialized.as_bytes())?;
    //     let start_pos = self.writer.stream_position()? - serialized.len() as u64;
    //     Ok(CommandPos::new(self.current_log_gen, start_pos, serialized.len() as u64))
    // }

    /// Create a new log file with given generation number and add the reader to the readers map.
    ///
    /// Returns the writer to the log.
    fn new_log_file(&mut self, gen: u64) -> Result<BufWriterWithPos<File>> {
        new_log_file(&self.working_dir, gen, &mut self.readers)
    }

    /// Clears stale entries in the command log.
    pub fn compact(&mut self) -> Result<()> {
        // increase current gen by 2. current_gen + 1 is for the compaction file.
        let compaction_gen = self.current_log_gen + 1;
        self.current_log_gen += 2;
        self.writer = self.new_log_file(self.current_log_gen)?;

        let mut compaction_writer = self.new_log_file(compaction_gen)?;

        let mut new_pos = 0; // pos in the new log file.

        // iterate over all CommandPos values in the index btree map an write them into the
        // compaction log
        for cmd_pos in &mut self.index.values_mut() {
            // get the reader for the generation log file the commandPos is pointing to
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");
            // seek to the position of the command within the log file
            if reader.pos != cmd_pos.pos {
                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            }

            // read the command from the log and copy it to the compaction log
            let mut entry_reader = reader.take(cmd_pos.len);
            let len = io::copy(&mut entry_reader, &mut compaction_writer)?;
            // update cmd_pos with its new location info within the (new) compaction log
            *cmd_pos = (compaction_gen, new_pos..new_pos + len).into();
            // increment the count of (bytes) that have been written in the compaction log
            new_pos += len;
        }

        compaction_writer.flush()?;

        // remove stale log files by comparing gen numbers with the current compaction gen
        let stale_gens: Vec<_> = self
            .readers
            .keys()
            .filter(|&&gen| gen < compaction_gen)
            .cloned()
            .collect();
        for stale_gen in stale_gens {
            self.readers.remove(&stale_gen);
            fs::remove_file(build_log_path(&self.working_dir, stale_gen))?;
        }
        self.uncompacted = 0;

        Ok(())
    }
}


/// loads the commands from the given reader into the given `index` map
/// returns the amount of bytes that could be compacted.
/// `gen` is the generation number of the file being read by `reader`
///
/// # Errors
/// IO Errors will be returned if any log file could not be opened/read
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &mut BTreeMap<String, CommandPos>) -> Result<u64>
{
    let mut pos= reader.seek(SeekFrom::Start(0))?;
    let mut uncompacted = 0_u64;
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

    while let Some(command) = stream.next() {
        let length = stream.byte_offset() as u64 - pos; // length of the command
        match command? {
            Command::Set { key, .. } => {

                if let Some(old_command) = index.insert(key, CommandPos::new(gen, pos as u64, length)) {
                    uncompacted += old_command.len;
                }
            },
            Command::Remove { key } => {
                if let Some(old_command) = index.remove(&key) {
                    uncompacted += old_command.len;
                }
                // this "remove" command itself can be deleted in the next compaction
                uncompacted += length;
            },
        }
        pos = stream.byte_offset() as u64;
    }

    Ok(uncompacted)
}

/// Constructs a log file path using the `gen` number as the file stem and the appending the
/// suffix ".log" to it. The log file name is then joined to the current working directory path
fn build_log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// Create a new log file with given generation number and add the reader to the readers map.
///
/// Returns the writer to the log.
fn new_log_file(
    path: &Path,
    gen: u64,
    readers: &mut HashMap<u64, BufReaderWithPos<File>>,
) -> Result<BufWriterWithPos<File>> {
    let path = build_log_path(&path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;

    readers.insert(gen, BufReaderWithPos::new(File::open(&path)?)?);
    Ok(writer)
}



/// These are the command types that will be recorded in the command log(s)
#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Set { key: String, value: String },
    Remove { key: String },
    //Get { key: String },
}





/// Holds position data for commands that have been written into a command log.
#[derive(Debug, Copy, Clone)]
struct CommandPos {
    // the log generation number
    gen: u64,
    // position of the command with the log (byte offset)
    pos: u64,
    // the total length of the command data
    len: u64,
}

impl CommandPos {
    /// builder method to construct a new `CommandPos`
    fn new(gen: u64, pos: u64, len: u64) -> Self {
        CommandPos { gen, pos, len }
    }
}

/// enables conversion from a tuple of (generation number, pos_start..pos_end) into
/// a `CommandPos`. The len of the command pos will be computed from the range
impl From<(u64, Range<u64>)> for CommandPos {
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}



/// returns the log generation numbers located in the given `dir` path, sorted in ascending order
///
/// # Errors
/// returns an IO Error if the given `dir` and/or log files in that dir could not be read,
/// or if a file stem could not be found for a .log file
/// or if a file stem could not be converted to an integer
fn get_log_gens(dir: &Path) -> Result<Option<Vec<u64>>> {
    let mut logs: Vec<u64> = vec![];
    for entry in (fs::read_dir(dir)?).flatten() {
        if entry.file_type()?.is_file() && entry.path().extension().map_or(false, |ext| ext.to_str() == Some("log"))
        {
            let stem = entry.path()
                .file_stem()
                .ok_or(anyhow!("could not find log file stem for {:?}", &entry.path()))?.to_os_string();
            let gen_str = stem.to_str().map(String::from).ok_or(anyhow!("could not convert file stem to &str: {:?}", &stem))?;
            let gen = gen_str.parse::<u64>().context(format!("could not parse {} into an integer", &gen_str))?;
            logs.push(gen);
        }

    }
    if !logs.is_empty() {
        logs.sort_unstable();
        Ok(Some(logs))
    }  else {
        Ok(None)
    }
}


/// A struct that holds a BufferedReader along with the current seek `pos` of that BufferedReader
#[derive(Debug)]
struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
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
        let pos = inner.seek(SeekFrom::Current(0))?;
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