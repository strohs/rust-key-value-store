use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use crate::error::Result;
use crate::KvsError;
use crate::KvsError::IOError;

// the default file name for the kvs command log
static DEFAULT_LOG_FILE_NAME: &str = "kvs.log";

/// the main structure used for working with a KvStore
#[derive(Debug)]
pub struct KvStore {
    store: HashMap<String, String>,

    // path to the command log, An on-disk sequence of commands, in the order originally received and executed
    log_path: PathBuf,

    // holds the in-memory index, a map of keys to the position of that key within the command log (a.k.a a log pointer)
    index: BTreeMap<String, CommandPos>,
}

impl KvStore {


    /// creates a KvStore using the data from the kvs logfile located in the `working_dir`
    /// If the kvs log does not yet exist, it will be created
    pub fn open(working_dir: impl Into<PathBuf>) -> Result<KvStore> {
        let log_path = working_dir.into().join(DEFAULT_LOG_FILE_NAME);

        if !log_path.exists() {
            File::create(&log_path)?;
        }

        Ok(KvStore {
            store: HashMap::new(),
            index: BTreeMap::new(),
            log_path,
        })
    }

    /// loads the commands from the kvs log file into the (in-memory) index
    pub fn load(&mut self) -> Result<()> {
        self.index.clear();
        let log_file = File::open(&self.log_path)?;

        let reader = BufReader::new(log_file);
        let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
        let mut pos: usize = 0;
        while let Some(command) = stream.next() {
            let length = (stream.byte_offset() - pos) as u64;
            match command? {
                Command::Set { key, .. } => {
                    self.index.insert(key, CommandPos::new(pos as u64, length));
                },
                Command::Remove { key } => {
                    self.index.remove(&key);
                },
            }
            pos += stream.byte_offset();
        }

        Ok(())
    }

    /// attempts to retrieve the value associated with `key`.
    /// returns `Some(value)` if the `key` was found, else returns `None`
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    /// ```
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        // load command map into index
        self.load()?;

        // check for existence of key in index
        if let Some(CommandPos { pos, len }) = self.index.get(&key) {
            // load value from log
            let mut reader = BufReader::new(File::open(&self.log_path)?);
            let mut buf: Vec<u8> = Vec::with_capacity(*len as usize);
            reader.seek(SeekFrom::Start(*pos))?;
            reader.read_exact(&mut buf)?;
            let value = String::from_utf8(buf)
                .map_err(|_e| IOError(format!("could not convert command at pos {} len {} into a valid UTF8 String", pos, len)))?;
            Ok(Some(value))
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
        let command = Command::Set { key: key.clone(), value: value.clone() };
        // write into the command log
        let pos = self.write(&command)?;
        // write into the index
        self.index.insert(key, pos);
        Ok(())
    }

    /// removes the specified `key` and its associated value from this KvStore
    /// # Examples
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// ```
    pub fn remove(&mut self, key: String) -> Result<()> {
        match self.index.get(&key) {
            Some(pos) => {
                let command = Command::Remove { key: key.clone() };
                self.write(&command)?;
                Ok(())
            },
            None => Err(KvsError::KeyNotFound),
        }
    }

    /// serializes `command` and writes it into the kvs command log
    /// returns a [`CommandPos`] indicating where the command was written at
    /// within the log
    fn write(&mut self, command: &Command) -> Result<CommandPos> {
        let mut log_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.log_path)?;

        let serialized = command.serialize()?;
        let mut buf_writer = BufWriter::new(&mut log_file);
        buf_writer.write_all(serialized.as_bytes())?;
        let start_pos = buf_writer.stream_position()? - serialized.len() as u64;
        Ok(CommandPos::new(start_pos, serialized.len() as u64))
    }
}



/// An enum that types the commands that can be issued to the KvStore
#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Set { key: String, value: String },
    Remove { key: String },
    //Get { key: String },
}

impl Command {

    /// serialize this command into a ['String']
    /// returns `Ok(String)` upon successful serialization,
    /// otherwise `Err(KvsError::SerializationError)`
    pub fn serialize(&self) -> Result<String> {
        let serialized = serde_json::to_string(&self)?;
        Ok(serialized)
    }
}




/// the position and length of a serialized command within the kv command log.
#[derive(Debug, Copy, Clone)]
struct CommandPos {
    //gen: u64,
    pos: u64,
    len: u64,
}

impl CommandPos {
    fn new(pos: u64, len: u64) -> Self {
        CommandPos { pos, len }
    }
}
