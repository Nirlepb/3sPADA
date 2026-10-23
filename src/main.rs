use clap::{Parser, Subcommand};
use merklelsm::{
    Db, Error, Options, Result, Transaction,
    manifest::{self, Manifest},
    sstable::SsTable,
};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::ExitCode,
};
mod presentation;

#[derive(Parser)]
#[command(version, about = "MerkleLSM: a durable educational audit ledger")]
struct Cli {
    #[arg(long, global = true, default_value = "demo-db")]
    db: PathBuf,
    #[arg(long, global = true, default_value_t = 1_048_576)]
    memtable_bytes: usize,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Open the interactive presentation menu with real storage traces.
    #[command(alias = "interactive", alias = "presentation")]
    Present,
    /// Create/open a database.
    Init,
    /// Append an immutable financial audit event. Amounts are integer INR paise.
    Record {
        #[arg(long)]
        id: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount_paise: i64,
        #[arg(long)]
        timestamp_ms: u64,
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Look up an immutable transaction by ID.
    Transaction {
        id: String,
    },
    /// List transactions in ID order.
    Transactions,
    /// Put a value in the separate mutable demonstration namespace.
    Put {
        key: String,
        value: String,
    },
    Get {
        key: String,
    },
    Delete {
        key: String,
    },
    /// List demonstration keys: inclusive start, exclusive end.
    Scan {
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
    },
    Flush,
    Compact,
    /// Verify every table and the WAL; optionally compare an external snapshot.
    Verify {
        #[arg(long)]
        checkpoint: Option<PathBuf>,
    },
    /// Flush and create an external trusted checkpoint (must be a new file).
    Checkpoint {
        output: PathBuf,
    },
    /// Export a block inclusion proof for a flushed transaction.
    Prove {
        id: String,
        output: PathBuf,
    },
    /// Verify a proof offline against an independently trusted checkpoint.
    VerifyProof {
        id: String,
        proof: PathBuf,
        checkpoint: PathBuf,
    },
    Stats,
    /// Deliberately corrupt one byte. Use only on a disposable demo database.
    Tamper {
        #[arg(long, default_value_t = 0)]
        table: usize,
        #[arg(long, default_value_t = 0)]
        block: usize,
        #[arg(long, required = true)]
        allow_demo_tamper: bool,
    },
}
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
fn run(cli: Cli) -> Result<()> {
    if let Command::VerifyProof {
        id,
        proof,
        checkpoint,
    } = &cli.command
    {
        if std::fs::metadata(proof)?.len() > 16 * 1024 * 1024 {
            return Err(Error::InvalidInput("proof JSON exceeds 16 MiB".into()));
        }
        let proof: merklelsm::proof::TransactionProof =
            serde_json::from_slice(&std::fs::read(proof)?)?;
        let transaction = proof.verify(id, &Manifest::read(checkpoint)?)?;
        println!("{}", serde_json::to_string_pretty(&transaction)?);
        return Ok(());
    }
    if matches!(cli.command, Command::Present) {
        return presentation::run(
            cli.db,
            Options {
                memtable_bytes: cli.memtable_bytes,
            },
        );
    }
    if let Command::Tamper {
        table,
        block,
        allow_demo_tamper,
    } = cli.command
    {
        if !allow_demo_tamper {
            return Err(Error::InvalidInput("pass --allow-demo-tamper".into()));
        }
        let _lock = manifest::lock(&cli.db)?;
        let manifest = Manifest::load(&cli.db)?;
        let meta = manifest
            .tables
            .get(table)
            .ok_or_else(|| Error::InvalidInput("table number out of range; flush first".into()))?;
        let path = cli.db.join(&meta.file);
        let table = SsTable::open(&path, meta)?;
        let offset = table.block_payload_offset(block)?;
        let mut file = OpenOptions::new().read(true).write(true).open(&path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut byte = [0];
        file.read_exact(&mut byte)?;
        byte[0] ^= 1;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&byte)?;
        file.sync_all()?;
        println!(
            "Deliberately corrupted {} data block {block} at byte {offset}",
            path.display()
        );
        return Ok(());
    }
    // Read commands must not create an empty database after a path typo.
    if !matches!(
        cli.command,
        Command::Init | Command::Record { .. } | Command::Put { .. }
    ) && !cli.db.join(manifest::MANIFEST_FILE).exists()
    {
        return Err(Error::InvalidInput(
            "database does not exist; run init, put, or record first".into(),
        ));
    }
    let mut db = Db::open_with_options(
        &cli.db,
        Options {
            memtable_bytes: cli.memtable_bytes,
        },
    )?;
    if db.stats().recovered_tail_bytes > 0 {
        eprintln!(
            "Recovered WAL: removed {} incomplete tail bytes",
            db.stats().recovered_tail_bytes
        );
    }
    match cli.command {
        Command::Init => println!("Database ready: {}", cli.db.display()),
        Command::Record {
            id,
            from,
            to,
            amount_paise,
            timestamp_ms,
            note,
        } => {
            let seq = db.record(Transaction {
                id,
                from,
                to,
                amount_paise,
                timestamp_ms,
                note,
            })?;
            println!("Recorded transaction at sequence {seq}");
        }
        Command::Transaction { id } => println!(
            "{}",
            serde_json::to_string_pretty(&db.get_transaction(&id)?)?
        ),
        Command::Transactions => println!("{}", serde_json::to_string_pretty(&db.transactions()?)?),
        Command::Put { key, value } => println!(
            "Stored at sequence {}",
            db.put(key.as_bytes(), value.as_bytes())?
        ),
        Command::Get { key } => {
            let value = db
                .get(key.as_bytes())?
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
            println!("{}", serde_json::to_string(&value)?);
        }
        Command::Delete { key } => println!("Deleted at sequence {}", db.delete(key.as_bytes())?),
        Command::Scan { start, end } => {
            for entry in db.scan(
                start.as_deref().map(str::as_bytes),
                end.as_deref().map(str::as_bytes),
            )? {
                println!(
                    "{}",
                    serde_json::json!({ "key":String::from_utf8_lossy(&entry.key), "value":String::from_utf8_lossy(&entry.value), "sequence":entry.seq })
                );
            }
        }
        Command::Flush => {
            db.flush()?;
            println!("Flushed durable SSTable and manifest");
        }
        Command::Compact => {
            db.compact()?;
            println!("Compaction complete");
        }
        Command::Verify { checkpoint } => match checkpoint {
            Some(path) => {
                db.verify_checkpoint(path)?;
                println!("Verified tables, WAL, and trusted checkpoint");
            }
            None => {
                db.verify()?;
                println!("Verified tables against local manifest and checked WAL CRCs");
            }
        },
        Command::Checkpoint { output } => {
            db.checkpoint(&output)?;
            println!("Checkpoint created: {}", output.display());
        }
        Command::Prove { id, output } => {
            let proof = db.transaction_proof(&id)?;
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output)?;
            file.write_all(&serde_json::to_vec_pretty(&proof)?)?;
            file.sync_all()?;
            println!("Transaction block proof exported");
        }
        Command::VerifyProof { .. } => unreachable!("handled before database open"),
        Command::Stats => println!("{}", serde_json::to_string_pretty(&db.stats())?),
        Command::Tamper { .. } | Command::Present => {
            unreachable!("handled before opening database")
        }
    }
    Ok(())
}
