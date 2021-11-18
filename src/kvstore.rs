use std::collections::{BTreeMap};
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
    // path to the command log, An on-disk sequence of commands, in the order originally received and executed
    log_path: PathBuf,

    // index holds all keys currently in the database.
    // It maps keys to their position within the command log
    index: BTreeMap<String, CommandPos>,
}

impl KvStore {

    /// create a new KvStore
    fn new(log_path: PathBuf) -> Self {
        KvStore {
            index: BTreeMap::new(),
            log_path
        }
    }

    /// creates a KvStore using the data from the kvs logfile located in the `working_dir`
    /// If the kvs log does not yet exist, it will be created
    pub fn open(working_dir: impl Into<PathBuf>) -> Result<KvStore> {
        let log_path = working_dir.into().join(DEFAULT_LOG_FILE_NAME);

        if !log_path.exists() {
            File::create(&log_path)?;
        }

        let mut kvs = KvStore::new(log_path);
        // load keys from the command log
        kvs.load()?;

        Ok(kvs)
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
            // ignore GET commands
            match command? {
                Command::Set { key, .. } => {
                    self.index.insert(key, CommandPos::new(pos as u64, length));
                },
                Command::Remove { key } => {
                    self.index.remove(&key);
                },
            }
            pos = stream.byte_offset();
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
        // check for existence of key in index
        if let Some(CommandPos { pos, len }) = self.index.get(&key) {
            // read the command string from the command log
            let mut reader = BufReader::new(File::open(&self.log_path)?);
            let mut buf: Vec<u8> = vec![0_u8; *len as usize];
            reader.seek(SeekFrom::Start(*pos))?;
            reader.read_exact(&mut buf)?;
            let command_string = String::from_utf8(buf)
                .map_err(|_e| IOError(format!("could not convert command at pos {} len {} into a valid UTF8 String", pos, len)))?;

            // deserialize the command string into a command enum and return the value field
            match serde_json::from_str::<Command>(&command_string)? {
                Command::Set { value, .. } => Ok(Some(value)),
                _ => Err(KvsError::SerializationError),
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
        let command = Command::Set { key: key.clone(), value: value.clone() };
        // write the command into the log
        let pos: CommandPos = self.write(&command)?;
        // insert the command's key and pos data into the index
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
        if self.index.contains_key(&key) {
            // creates a value representing the "rm" command, containing its key
            let command = Command::Remove { key: key.clone() };
            // append the serialized command to the log
            self.write(&command)?;
            // exits silently with error code 0
            // todo should we remove key from index here?
            self.index.remove(&key);

            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }

    /// serializes the `command` and writes it into the kvs command log file
    /// returns a [`CommandPos`] indicating where the command was written at within the log
    fn write(&mut self, command: &Command) -> Result<CommandPos> {
        let mut log_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.log_path)?;

        let serialized = command.serialize()?;
        let mut writer = BufWriter::new(&mut log_file);
        writer.write_all(serialized.as_bytes())?;
        let start_pos = writer.stream_position()? - serialized.len() as u64;

        Ok(CommandPos::new(start_pos, serialized.len() as u64))
    }

    fn compact() {
        // could be triggered during a set AND when log size reaches a pre-set size threshold
        // could create a new command log file to hold the newly compacted log, once new log safely
        //  written, we delete the old one or possible keep the old one
        // new log will only contain non-duplicate SET commands for a key
        // OPT 1: iterate current keys of index, serialize them, compute new commandPos,
        //   write into new log, update the (in-mem) index with new commandPos
        unimplemented!()
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
