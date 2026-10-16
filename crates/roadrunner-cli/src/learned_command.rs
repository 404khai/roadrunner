//! Source-verified interchange; Python fitting never enters operational authority.
use roadrunner_dataset::{learned::export_research, load_dataset};
use std::path::Path;

pub(super) fn run(args: &[String]) -> Result<(), String> {
    match args {
        [operation, directory] if operation == "export" => {
            let dataset = load_dataset(Path::new(directory)).map_err(|e| e.to_string())?;
            let export = export_research(&dataset).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&export).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        _ => Err("usage: roadrunner learned export <dataset-directory>".into()),
    }
}
