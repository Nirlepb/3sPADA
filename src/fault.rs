//! Opt-in process-local hooks. Environment variables have no effect in default builds.
use crate::Result;
pub fn hit(stage: &str) -> Result<()> {
    if std::env::var("MERKLELSM_FAULT_STAGE").as_deref() != Ok(stage) {
        return Ok(());
    }
    match std::env::var("MERKLELSM_FAULT_ACTION").as_deref() {
        Ok("exit") => std::process::exit(86), // No destructors run.
        Ok("error") => Err(std::io::Error::other(format!("injected failure: {stage}")).into()),
        _ => Ok(()),
    }
}
