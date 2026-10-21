//! Command-line entry point for Roadrunner.

use std::ffi::OsString;
use std::process::ExitCode;

use roadrunner_osm::{
    compile_motorcycle_graph, decode_dataset_artifact, extract_pbf, load_snapshot_bundle,
    write_dataset_artifact_atomic, write_snapshot_bundle_atomic,
};

mod alternatives_command;
mod baseline_command;
mod command_options;
mod dataset_command;
mod demo_command;
mod graph_command;
mod learned_command;
mod reference_comparison;
mod route_command;
mod simulation_command;
mod terminal;
mod time_dependent_command;
mod traffic_command;

fn main() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = arguments
        .first()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let subcommand = arguments
        .get(1)
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let json = arguments.iter().any(|arg| arg == "--json")
        && (command == "demo"
            || (command == "graph" && matches!(subcommand, "stats" | "import"))
            || (command == "route" && (subcommand == "compare" || subcommand.starts_with("--"))));
    match run(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            if json {
                eprintln!("{}", serde_json::json!({"error": message}));
            } else {
                eprintln!("error: {message}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<OsString>) -> Result<(), String> {
    let args: Vec<_> = arguments
        .into_iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    if args.first().is_some_and(|s| s == "dataset") {
        return dataset_command::run(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "baseline") {
        return baseline_command::run(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "learned") {
        return learned_command::run(&args[1..]);
    }
    run_commands(&args)
}

fn run_commands(args: &[String]) -> Result<(), String> {
    match args {
        [flag] if flag == "--version" || flag == "-V" => {
            println!("roadrunner {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [graph, stats, path, rest @ ..] if graph == "graph" && stats == "stats" => {
            graph_command::stats(path, rest)
        }
        [graph, import, input, output, rest @ ..] if graph == "graph" && import == "import" => {
            graph_command::import(input, output, rest)
        }
        [route, compare, rest @ ..] if route == "route" && compare == "compare" => {
            route_command::run(rest, true)
        }
        [route, first, rest @ ..] if route == "route" && first.starts_with("--") => {
            let mut options = vec![first.clone()];
            options.extend_from_slice(rest);
            route_command::run(&options, false)
        }
        [command, rest @ ..] if command == "serve" => serve(rest),
        [command, rest @ ..] if command == "demo" => demo_command::run(rest),
        _ => run_legacy_commands(args),
    }
}

fn run_legacy_commands(args: &[String]) -> Result<(), String> {
    match args {
        [command] if command == "--help" || command == "-h" || command == "help" => {
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
        [
            route,
            traffic,
            snapshot,
            source,
            destination,
            scenario_flag,
            scenario,
        ] if route == "route" && traffic == "traffic" && scenario_flag == "--scenario" => {
            traffic_command::run(snapshot, source, destination, scenario)
        }
        [
            route,
            schedule,
            snapshot,
            source,
            destination,
            scenario_flag,
            scenario,
            depart_flag,
            depart,
        ] if route == "route"
            && schedule == "schedule"
            && scenario_flag == "--scenario"
            && depart_flag == "--depart" =>
        {
            time_dependent_command::run(snapshot, source, destination, scenario, depart)
        }
        [benchmark, dispatch, scenario, rest @ ..]
            if benchmark == "benchmark" && dispatch == "dispatch" =>
        {
            simulation_command::compare_arguments(scenario, rest)
        }
        [simulate, scenario] if simulate == "simulate" => simulation_command::run(scenario, false),
        [simulate, scenario, json] if simulate == "simulate" && json == "--json" => {
            simulation_command::run(scenario, true)
        }
        [] => {
            print_help();
            Ok(())
        }
        _ => Err("invalid arguments; run `roadrunner --help`".to_owned()),
    }
}

fn serve(args: &[String]) -> Result<(), String> {
    let binary = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name(format!("roadrunner-api{}", std::env::consts::EXE_SUFFIX));
    if !binary.is_file() {
        return Err(format!(
            "API executable missing at {}; build roadrunner-api separately and place it beside roadrunner",
            binary.display()
        ));
    }
    let mut command = std::process::Command::new(binary);
    command.args(args);
    // Replace the CLI so shutdown signals and exit status belong to the API itself.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(command.exec().to_string())
    }
    #[cfg(not(unix))]
    {
        let status = command.status().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("API exited with {status}"))
        }
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
        "{}\nRouting & delivery optimization · v{}\n",
        terminal::BANNER,
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "Core workflows:\n  roadrunner demo [--riders 12|24] [--graph <pinned-path>] [--json]\n  roadrunner graph import <input.osm.pbf> <new-snapshot-directory> --source-id <identity> [--json]\n  roadrunner graph stats <snapshot-directory-or-graph-artifact> [--json]\n  roadrunner route [compare] --graph <path> --from <lat,lng|node:id|osm:id> --to <endpoint> [--json]\n    [--algorithm dijkstra|astar] [--alternatives <n>] [--snap-radius-meters <m>]\n  roadrunner serve [API arguments] (requires sibling roadrunner-api executable)\n  Coordinates snap to the nearest node within 1000m by default, not to road edges.\n  New route commands use motorcycle free-flow travel time. Compare runs both algorithms.\n  New graph/routing --json emits only JSON; errors are JSON on stderr with a nonzero exit.\n"
    );
    println!(
        "Existing commands:\n  \
         roadrunner osm extract <input.osm.pbf> <output.rr-osm> --source-id <identity>\n  \
         roadrunner osm compile <input.rr-osm> <snapshot-directory>\n  \
         roadrunner graph verify <snapshot-directory> --deep\n  \
         roadrunner route corpus <snapshot-directory> <route-corpus.json>\n  \
         roadrunner route alternatives <snapshot-directory> <from-osm-node> <to-osm-node> --count <n>\n  \
         roadrunner route traffic <snapshot-directory> <from-osm-node> <to-osm-node> --scenario <traffic.json>\n  \
         roadrunner route schedule <snapshot-directory> <from-osm-node> <to-osm-node> --scenario <profile.json> --depart <seconds>\n  \
         roadrunner baseline evaluate <dataset-directory> <new-result-directory>\n  \
         roadrunner baseline verify <result-directory>\n  \
         roadrunner learned export <dataset-directory>\n  \
         roadrunner dataset build <request.json> <new-output-directory>\n  \
         roadrunner dataset verify <dataset-directory>\n  \
         roadrunner simulate <scenario.json> [--json]\n  \
         roadrunner benchmark dispatch <scenario.json> [--idle-penalty-weight <weight>] [--json]"
    );
}
