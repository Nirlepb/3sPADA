//! Run: cargo run --example basic -- /tmp/my-ledger
use merklelsm::{Db, Result, Transaction};
fn main() -> Result<()> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "demo-db".into());
    let mut db = Db::open(path)?;
    let transaction = Transaction {
        id: "example-001".into(),
        from: "Alice".into(),
        to: "Bob".into(),
        amount_paise: 1050,
        timestamp_ms: 1700000000000,
        note: "Lunch reimbursement".into(),
    };
    if db.get_transaction(&transaction.id)?.is_none() {
        db.record(transaction)?;
    }
    db.flush()?;
    db.verify()?;
    println!("{:#?}", db.get_transaction("example-001")?);
    Ok(())
}
