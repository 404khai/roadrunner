//! Command-line entry point for Roadrunner.

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

use roadrunner_core::graph::write_graph_artifact_atomic;
use roadrunner_osm::{
    compile_motorcycle_graph, decode_dataset_artifact, extract_pbf, write_dataset_artifact_atomic,
};

static TEMPORARY_MANIFEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<OsString>) -> Result<(), String> {
    let args: Vec<_> = arguments
        .into_iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    match args.as_slice() {
        [command] if command == "--help" || command == "help" => {
            print_help();
            Ok(())
        }
        [osm, extract, input, output, source_flag, source_id]
            if osm == "osm" && extract == "extract" && source_flag == "--source-id" =>
        {
            let dataset = extract_pbf(input, source_id).map_err(|error| error.to_string())?;
            write_dataset_artifact_atomic(output, &dataset).map_err(|error| error.to_string())?;
            println!(
                "normalized {} nodes, {} ways, and {} restrictions into {}",
                dataset.nodes.len(),
                dataset.ways.len(),
                dataset.relations.len(),
                output
            );
            Ok(())
        }
        [osm, compile, input, graph, manifest] if osm == "osm" && compile == "compile" => {
            compile_dataset(input, graph, manifest)
        }
        [] => {
            print_help();
            Ok(())
        }
        _ => Err("invalid arguments; run `roadrunner --help`".to_owned()),
    }
}

fn compile_dataset(input: &str, graph: &str, manifest: &str) -> Result<(), String> {
    let bytes = std::fs::read(input)
        .map_err(|error| format!("could not read normalized dataset {input}: {error}"))?;
    let decoded = decode_dataset_artifact(&bytes).map_err(|error| error.to_string())?;
    let compiled = compile_motorcycle_graph(&decoded).map_err(|error| error.to_string())?;
    write_graph_artifact_atomic(Path::new(graph), &compiled.graph)
        .map_err(|error| error.to_string())?;
    let manifest_bytes = serde_json::to_vec_pretty(&compiled.manifest)
        .map_err(|error| format!("could not encode build manifest: {error}"))?;
    write_atomic(Path::new(manifest), &manifest_bytes)?;
    println!(
        "compiled {} nodes, {} segments, and {} directed edges into {}",
        compiled.graph.node_count(),
        compiled.graph.segment_count(),
        compiled.graph.edge_count(),
        graph
    );
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| "manifest path requires a file name".to_owned())?;
    let sequence = TEMPORARY_MANIFEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary: PathBuf = path.with_file_name(format!(
        ".{}.tmp-{}-{sequence}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("could not create temporary manifest: {error}"))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("could not write temporary manifest: {error}"));
    }
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("could not publish manifest: {error}"));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("could not synchronize manifest directory: {error}"))?;
    Ok(())
}

fn print_help() {
    println!(
        "Roadrunner\n\n\
         Usage:\n  \
         roadrunner osm extract <input.osm.pbf> <output.rr-osm> --source-id <identity>\n  \
         roadrunner osm compile <input.rr-osm> <output.rr-graph> <manifest.json>"
    );
}
