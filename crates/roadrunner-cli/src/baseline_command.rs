//! Offline original-B0 evaluation against frozen, source-verified Phase 23 evidence.
use roadrunner_dataset::{
    baseline::{build_baseline_artifact, load_baseline_artifact, publish_baseline_artifact},
    load_dataset,
};
use std::path::Path;

pub(super) fn run(args: &[String]) -> Result<(), String> {
    match args {
        [operation,dataset,output] if operation=="evaluate" => {
            let input=load_dataset(Path::new(dataset)).map_err(|e|e.to_string())?;
            let artifact=build_baseline_artifact(input,crate::dataset_command::reproduction()?).map_err(|e|e.to_string())?;
            let manifest=publish_baseline_artifact(&artifact,Path::new(output)).map_err(|e|e.to_string())?;
            println!("{}",serde_json::to_string_pretty(&serde_json::json!({"manifest":manifest,"report":artifact.report})).map_err(|e|e.to_string())?);
            Ok(())
        }
        [operation,directory] if operation=="verify" => {
            let artifact=load_baseline_artifact(Path::new(directory)).map_err(|e|e.to_string())?;
            println!("verified {}: {} committed starts; {} exact pairs; descriptive synthetic B0 only",artifact.report.schema,artifact.report.descriptive_all_included.coverage.committed_starts,artifact.report.descriptive_all_included.coverage.prediction_on_eligible_coverage.numerator);
            Ok(())
        }
        _=>Err("usage: roadrunner baseline evaluate <dataset-directory> <new-result-directory> | baseline verify <result-directory>".into()),
    }
}
