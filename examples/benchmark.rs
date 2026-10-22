//! An illustrative timing run, not a statistical benchmark or throughput claim.
use merklelsm::{Db, Error, Options, Result};
use std::{path::PathBuf, time::Instant};
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().unwrap_or_else(|| "bench-db".into()));
    let count: u32 = args
        .next()
        .unwrap_or_else(|| "1000".into())
        .parse()
        .map_err(|_| Error::InvalidInput("count must be an integer".into()))?;
    if count == 0 || dir.exists() {
        return Err(Error::InvalidInput(
            "use a new directory and a positive count".into(),
        ));
    }
    let mut db = Db::open_with_options(
        &dir,
        Options {
            memtable_bytes: 64 * 1024,
        },
    )?;
    let start = Instant::now();
    for i in 0..count {
        db.put(&i.to_be_bytes(), &[42; 128])?;
    }
    let writes = start.elapsed();
    let start = Instant::now();
    db.flush()?;
    let flush = start.elapsed();
    let start = Instant::now();
    for i in 0..count {
        if db.get(&i.to_be_bytes())?.is_none() {
            return Err(Error::Corruption("benchmark read missed a key".into()));
        }
    }
    let reads = start.elapsed();
    let start = Instant::now();
    db.compact()?;
    let compact = start.elapsed();
    db.verify()?;
    println!(
        "{count} records; each write includes sync_all\nwrites: {writes:?}\nflush: {flush:?}\nreads: {reads:?}\ncompact: {compact:?}"
    );
    println!("Database retained at {}", dir.display());
    Ok(())
}
