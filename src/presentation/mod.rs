//! Terminal presentation; all storage operations use the real library.
mod cases;
mod views;

use merklelsm::{
    Db, Error, Options, Result, Transaction, manifest::Manifest, merkle, sstable::SsTable,
};
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn run(path: PathBuf, options: Options) -> Result<()> {
    let db = Db::open_with_options(&path, options)?;
    db.enable_trace();
    println!(
        "\nMerkleLSM LIVE PRESENTATION\nDatabase: {}\nInputs operate on the real Rust storage engine. Amounts are integer INR paise.",
        path.display()
    );
    views::stats(&db);
    let mut session = Session {
        db: Some(db),
        path,
        options,
        input: Input { closed: false },
        workspace: None,
        serial: 0,
    };
    loop {
        println!(
            "\n---------------- MENU ----------------\n 1  Record a financial transaction\n 2  Find a transaction (show read path)\n 3  Inspect memtable, skip-list levels, and WAL\n 4  Flush memtable to a durable SSTable\n 5  Inspect SSTable blocks, index, Bloom filter, and Merkle tree\n 6  Verify current database\n 7  Put/update a demonstration key\n 8  Get a demonstration key (show read path)\n 9  Delete a demonstration key\n10  Scan demonstration keys (range)\n11  Compact all tables\n12  Close and reopen: demonstrate WAL recovery\n13  Run real edge-case exercises in a separate database\n14  Tamper with a COPY and detect its damaged block\n15  Export a trusted checkpoint\n16  Verify against a checkpoint\n17  Show statistics and list transactions\n 0  Exit"
        );
        let choice = session.input.ask("Choose an option: ")?;
        if session.input.closed || choice.trim() == "0" {
            break;
        }
        if let Some(db) = &session.db {
            db.take_trace();
        }
        let outcome = session.dispatch(choice.trim());
        if session.input.closed {
            break;
        }
        if let Some(db) = &session.db {
            let events = db.take_trace();
            if !events.is_empty() {
                println!("\nUNDER THE HOOD (events from this operation):");
                for (i, event) in events.iter().enumerate() {
                    println!("  {}. {event}", i + 1);
                }
            }
        }
        if let Err(error) = outcome {
            println!(
                "Operation failed: {error}\nYou can correct the input and try again. After a write I/O failure, choose 12 to reopen."
            );
        }
    }
    println!("Session closed. Saved writes remain durable; use the same --db path to reopen.");
    Ok(())
}

struct Input {
    closed: bool,
}
impl Input {
    fn ask(&mut self, prompt: &str) -> Result<String> {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            self.closed = true;
            return Ok(String::new());
        }
        Ok(line.trim_end_matches(['\r', '\n']).to_owned())
    }
    fn field(&mut self, prompt: &str) -> Result<String> {
        let text = self.ask(prompt)?;
        if self.closed {
            return Err(Error::InvalidInput("input closed".into()));
        }
        Ok(text)
    }
    fn number<T: std::str::FromStr>(&mut self, prompt: &str, default: Option<T>) -> Result<T> {
        let text = self.field(prompt)?;
        if text.trim().is_empty() {
            return default.ok_or_else(|| Error::InvalidInput("enter a whole number".into()));
        }
        text.trim().parse().map_err(|_| {
            Error::InvalidInput("enter a whole number within the supported range".into())
        })
    }
}

struct Session {
    db: Option<Db>,
    path: PathBuf,
    options: Options,
    input: Input,
    workspace: Option<PathBuf>,
    serial: u64,
}
impl Session {
    fn db(&self) -> Result<&Db> {
        self.db.as_ref().ok_or(Error::Poisoned)
    }
    fn db_mut(&mut self) -> Result<&mut Db> {
        self.db.as_mut().ok_or(Error::Poisoned)
    }
    fn artifacts(&mut self, label: &str) -> Result<PathBuf> {
        if self.workspace.is_none() {
            let directory = fs::canonicalize(&self.path)?;
            let parent = directory.parent().ok_or_else(|| {
                Error::InvalidInput(
                    "database has no parent directory for presentation artifacts".into(),
                )
            })?;
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| Error::InvalidInput("system clock precedes the Unix epoch".into()))?
                .as_nanos();
            let base = parent.join("presentation-artifacts");
            fs::create_dir_all(&base)?;
            let root = base.join(format!("session-{}-{time}", std::process::id()));
            fs::create_dir(&root)?;
            println!("Presentation artifacts retained at {}", root.display());
            self.workspace = Some(root);
        }
        self.serial += 1;
        self.workspace
            .as_ref()
            .map(|root| root.join(format!("{label}-{}", self.serial)))
            .ok_or_else(|| Error::InvalidInput("missing presentation workspace".into()))
    }
    fn dispatch(&mut self, choice: &str) -> Result<()> {
        match choice {
            "1" => self.record(),
            "2" => {
                let id = self.input.field("Transaction ID: ")?;
                match self.db()?.get_transaction(&id)? {
                    Some(tx) => println!("FOUND:\n{}", serde_json::to_string_pretty(&tx)?),
                    None => println!("NOT FOUND: no transaction with ID {id:?}"),
                }
                Ok(())
            }
            "3" => views::memory(self.db()?, &self.path),
            "4" => {
                let before = self.db()?.stats();
                self.db_mut()?.flush()?;
                println!(
                    "Flush complete: memtable {} -> {} entries; tables {} -> {}",
                    before.memtable_entries,
                    self.db()?.stats().memtable_entries,
                    before.sstables,
                    self.db()?.stats().sstables
                );
                Ok(())
            }
            "5" => views::tables(self.db()?),
            "6" => {
                self.db()?.verify()?;
                println!(
                    "PASS: current tables match the local manifest; WAL frame checksums are valid.\nUse option 16 for comparison with an independently retained checkpoint."
                );
                Ok(())
            }
            "7" => {
                let key = self.input.field("Key: ")?;
                let value = self.input.field("Value (empty is allowed): ")?;
                let seq = self.db_mut()?.put(key.as_bytes(), value.as_bytes())?;
                println!("Stored key {key:?} at sequence {seq} in the k: demonstration namespace.");
                Ok(())
            }
            "8" => {
                let key = self.input.field("Key: ")?;
                match self.db()?.get(key.as_bytes())? {
                    Some(value) => println!("FOUND: {:?}", String::from_utf8_lossy(&value)),
                    None => println!("NOT FOUND: {key:?} (absent or deleted)"),
                }
                Ok(())
            }
            "9" => {
                let key = self.input.field("Key to delete: ")?;
                let seq = self.db_mut()?.delete(key.as_bytes())?;
                println!(
                    "Tombstone written at sequence {seq}. It stops reads from returning an older table value."
                );
                Ok(())
            }
            "10" => {
                let start = self
                    .input
                    .field("Start key, inclusive (Enter = unbounded): ")?;
                let end = self
                    .input
                    .field("End key, exclusive (Enter = unbounded): ")?;
                let entries = self.db()?.scan(
                    (!start.is_empty()).then_some(start.as_bytes()),
                    (!end.is_empty()).then_some(end.as_bytes()),
                )?;
                println!(
                    "Scan merges versions by greatest sequence and excludes tombstones. {} visible keys:",
                    entries.len()
                );
                views::entries(&entries);
                Ok(())
            }
            "11" => {
                let before = self.db()?.stats();
                self.db_mut()?.compact()?;
                let after = self.db()?.stats();
                println!(
                    "Compaction complete: {} tables -> {}; persisted sequence {}.\nAll active tables participated, so obsolete versions and tombstones can be removed.",
                    before.sstables, after.sstables, after.persisted_sequence
                );
                Ok(())
            }
            "12" => self.reopen(),
            "13" => {
                let root = self.artifacts("edge-cases")?;
                cases::run(&root)
            }
            "14" => self.tamper_copy(),
            "15" => {
                let suggested = self.artifacts("checkpoint")?.with_extension("json");
                let supplied = self.input.field(&format!(
                    "Checkpoint path (Enter = {}): ",
                    suggested.display()
                ))?;
                let path = if supplied.is_empty() {
                    suggested
                } else {
                    PathBuf::from(supplied)
                };
                self.db_mut()?.checkpoint(&path)?;
                println!(
                    "Checkpoint created: {}\nKeep this exact snapshot under separate control. Later writes/compaction also change the snapshot.",
                    path.display()
                );
                Ok(())
            }
            "16" => {
                let path = self.input.field("Trusted checkpoint path: ")?;
                self.db()?.verify_checkpoint(path)?;
                println!("PASS: current flushed database matches the exact trusted checkpoint.");
                Ok(())
            }
            "17" => {
                views::stats(self.db()?);
                let transactions = self.db()?.transactions()?;
                println!(
                    "{} financial transactions (ordered by ID):",
                    transactions.len()
                );
                for tx in transactions.iter().take(30) {
                    println!("{}", serde_json::to_string(tx)?);
                }
                if transactions.len() > 30 {
                    println!(
                        "Showing the first 30; use the transactions CLI command for all records."
                    );
                }
                Ok(())
            }
            _ => {
                println!("Choose one of the displayed numbers, or 0 to exit.");
                Ok(())
            }
        }
    }
    fn record(&mut self) -> Result<()> {
        let id = self.input.field("Transaction ID: ")?;
        let from = self.input.field("Sender: ")?;
        let to = self.input.field("Receiver: ")?;
        let amount_paise: i64 = self
            .input
            .number("Amount in integer paise (1050 = INR 10.50): ", None)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::InvalidInput("system clock precedes the Unix epoch".into()))?
            .as_millis();
        let now =
            u64::try_from(now).map_err(|_| Error::InvalidInput("timestamp overflow".into()))?;
        let timestamp_ms = self.input.number(
            "Unix timestamp in milliseconds (Enter = current time): ",
            Some(now),
        )?;
        let note = self.input.field("Note (optional): ")?;
        let before = self.db()?.stats().last_sequence;
        let outcome = self.db_mut()?.record(Transaction {
            id,
            from,
            to,
            amount_paise,
            timestamp_ms,
            note,
        });
        match outcome {
            Ok(seq) => {
                println!(
                    "RECORDED at sequence {seq}. The receipt is an audit event; amounts are stored as whole paise."
                );
                Ok(())
            }
            Err(e) => {
                println!(
                    "Sequence before: {before}; after: {}",
                    self.db()?.stats().last_sequence
                );
                Err(e)
            }
        }
    }
    fn reopen(&mut self) -> Result<()> {
        let before = self.db.as_ref().map(Db::stats);
        drop(self.db.take());
        println!("Closed handle WITHOUT flushing. Opening the same database now...");
        let db = Db::open_with_options(&self.path, self.options)?;
        db.enable_trace();
        let after = db.stats();
        println!(
            "Manifest persisted sequence: {}; recovered memtable entries: {}; last sequence: {}; incomplete tail bytes removed: {}",
            after.persisted_sequence,
            after.memtable_entries,
            after.last_sequence,
            after.recovered_tail_bytes
        );
        if let Some(before) = before {
            println!(
                "Before reopen: {} memtable entries; sequence {}",
                before.memtable_entries, before.last_sequence
            );
        }
        println!(
            "This exercise reopens normally; tests/crash_process.rs separately kills a writer process."
        );
        self.db = Some(db);
        Ok(())
    }
    fn tamper_copy(&mut self) -> Result<()> {
        println!(
            "This exercise flushes your pending entries, then corrupts a separate table COPY."
        );
        self.db_mut()?.flush()?;
        let manifest = Manifest::load(&self.path)?;
        if manifest.tables.is_empty() {
            return Err(Error::InvalidInput(
                "record a transaction or put a key first".into(),
            ));
        }
        for (i, meta) in manifest.tables.iter().enumerate() {
            println!("Table {i}: {}", meta.file);
        }
        let number: usize = self.input.number("Table number (Enter = 0): ", Some(0))?;
        let meta = manifest
            .tables
            .get(number)
            .ok_or_else(|| Error::InvalidInput("table number out of range".into()))?;
        let source = self.path.join(&meta.file);
        let table = SsTable::open(&source, meta)?;
        println!(
            "{} data blocks available (numbered from 0).",
            table.block_count()
        );
        let block: usize = self.input.number("Block number (Enter = 0): ", Some(0))?;
        let offset = table.block_payload_offset(block)?;
        let root = self.artifacts("tamper-copy")?;
        fs::create_dir(&root)?;
        let copy = root.join(&meta.file);
        fs::copy(&source, &copy)?;
        let mut file = fs::OpenOptions::new().read(true).write(true).open(&copy)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut byte = [0];
        file.read_exact(&mut byte)?;
        let original = byte[0];
        byte[0] ^= 1;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&byte)?;
        file.sync_all()?;
        let inspection = table.inspect()?;
        let block_info = &inspection.blocks[block];
        let changed = fs::read(&copy)?;
        let start = block_info.offset as usize;
        let end = start + 4 + block_info.payload_bytes as usize;
        println!(
            "COPY: {}\nFlipped byte {offset}: {original:#04x} -> {:#04x}\nExpected block leaf: {}\nChanged block leaf:  {}",
            copy.display(),
            byte[0],
            block_info.leaf_hash,
            merkle::hex(&merkle::hash_leaf(&changed[start..end]))
        );
        match SsTable::open(&copy, meta) {
            Err(Error::TamperDetected(message)) => println!("PASS: tamper detected: {message}"),
            Err(e) => return Err(e),
            Ok(_) => {
                return Err(Error::Corruption(
                    "tamper exercise unexpectedly accepted the changed block".into(),
                ));
            }
        }
        self.db()?.verify()?;
        println!("Original database verification: PASS. Continue using the menu.");
        Ok(())
    }
}
