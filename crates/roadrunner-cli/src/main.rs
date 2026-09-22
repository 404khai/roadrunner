//! Command-line entry point for Roadrunner.

use std::ffi::OsString;
use std::process::ExitCode;

use roadrunner_osm::{
    compile_motorcycle_graph, decode_dataset_artifact, extract_pbf, load_snapshot_bundle,
    write_dataset_artifact_atomic, write_snapshot_bundle_atomic,
};

mod alternatives_command;
mod reference_comparison;

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
        [osm, compile, input, snapshot] if osm == "osm" && compile == "compile" => {
            compile_dataset(input, snapshot)
        }
        [graph, verify, snapshot, deep]
            if graph == "graph" && verify == "verify" && deep == "--deep" =>
        {
            let loaded = load_snapshot_bundle(snapshot, true).map_err(|error| error.to_string())?;
            println!(
                "verified snapshot {}: {} nodes, {} segments, {} directed edges",
                loaded.manifest.graph_snapshot_digest,
                loaded.graph.node_count(),
                loaded.graph.segment_count(),
                loaded.graph.edge_count()
            );
            Ok(())
        }
        [route, corpus, snapshot, queries] if route == "route" && corpus == "corpus" => {
            reference_comparison::run(snapshot, queries)
        }
        [
            route,
            alternatives,
            snapshot,
            source,
            destination,
            count_flag,
            count,
        ] if route == "route" && alternatives == "alternatives" && count_flag == "--count" => {
            alternatives_command::run(snapshot, source, destination, count)
        }
        [] => {
            print_help();
            Ok(())
        }
        _ => Err("invalid arguments; run `roadrunner --help`".to_owned()),
    }
}

fn compile_dataset(input: &str, snapshot: &str) -> Result<(), String> {
    let bytes = std::fs::read(input)
        .map_err(|error| format!("could not read normalized dataset {input}: {error}"))?;
    let decoded = decode_dataset_artifact(&bytes).map_err(|error| error.to_string())?;
    let compiled = compile_motorcycle_graph(&decoded).map_err(|error| error.to_string())?;
    write_snapshot_bundle_atomic(snapshot, &compiled).map_err(|error| error.to_string())?;
    println!(
        "compiled {} nodes, {} segments, and {} directed edges into {}",
        compiled.graph.node_count(),
        compiled.graph.segment_count(),
        compiled.graph.edge_count(),
        snapshot
    );
    Ok(())
}

fn print_help() {
    println!(
        "Roadrunner\n\n\
         Usage:\n  \
         roadrunner osm extract <input.osm.pbf> <output.rr-osm> --source-id <identity>\n  \
         roadrunner osm compile <input.rr-osm> <snapshot-directory>\n  \
         roadrunner graph verify <snapshot-directory> --deep\n  \
         roadrunner route corpus <snapshot-directory> <route-corpus.json>\n  \
         roadrunner route alternatives <snapshot-directory> <from-osm-node> <to-osm-node> --count <n>"
    );
}
