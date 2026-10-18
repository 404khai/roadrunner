//! Graph-qualified Phase 34 headless execution. Full evidence is streamed, never truncated.
use roadrunner_core::{
    geo::{CanonicalCoordinate, KilometersPerHour},
    graph::{
        AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph,
        GraphBuildIdentity, GraphBuilder, GraphMetadata, GraphSnapshotId, decode_graph_artifact,
        encode_graph_artifact,
    },
};
use roadrunner_simulation::{
    SimulationEvent, SimulationScenario, simulate_observed, simulate_with_run_identity,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn grid(side: u32, spacing: i32) -> Result<FrozenGraph> {
    let mut b = GraphBuilder::new(
        GraphSnapshotId::new(34),
        GraphMetadata::new(
            "phase34-synthetic/v1",
            "synthetic",
            GraphBuildIdentity::new("phase34-grid", "v1", "v1", "bidirectional-orthogonal"),
        ),
    );
    let point = |id: u32| {
        CanonicalCoordinate::new(
            65_000_000 + i32::try_from(id / side).unwrap_or(0) * spacing,
            34_000_000 + i32::try_from(id % side).unwrap_or(0) * spacing,
        )
    };
    for id in 0..side * side {
        b.add_node(BuilderNodeId::new(u64::from(id)), point(id)?)?;
    }
    let properties = EdgeProperties::new(KilometersPerHour::new(36.0)?, None, AccessClass::General);
    let mut segment = 0;
    for from in 0..side * side {
        for to in [
            ((from % side) + 1 < side).then_some(from + 1),
            (from / side + 1 < side).then_some(from + side),
        ]
        .into_iter()
        .flatten()
        {
            b.add_segment(
                BuilderSegmentId::new(segment),
                BuilderNodeId::new(u64::from(from)),
                BuilderNodeId::new(u64::from(to)),
                vec![point(from)?, point(to)?],
                Some(properties),
                Some(properties),
            )?;
            segment += 1;
        }
    }
    Ok(b.finalize()?.with_forbidden_maneuvers(Vec::new())?)
}

struct EvidenceWriter {
    file: BufWriter<File>,
    hash: Sha256,
    bytes: u64,
}
impl Write for EvidenceWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(bytes)?;
        self.hash.update(&bytes[..n]);
        self.bytes += u64::try_from(n).unwrap_or(u64::MAX);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("inspect") => {
            let bytes = std::fs::read(args.get(2).ok_or("artifact")?)?;
            let graph = decode_graph_artifact(&bytes)?;
            println!("{}", json!({"graph_digest":graph.metadata().snapshot_digest(),"artifact_sha256":hash(&bytes),"nodes":graph.node_count(),"edges":graph.edge_count(),"generator":"external-artifact"}));
        }
        Some("grid") => {
            let side: u32 = args.get(2).ok_or("side")?.parse()?;
            let spacing: i32 = args.get(3).ok_or("spacing E7")?.parse()?;
            if !(2..=128).contains(&side) || !(100..=100_000).contains(&spacing) { return Err("unsafe grid dimensions".into()); }
            let graph = grid(side, spacing)?;
            let path = args.get(4).ok_or("artifact output")?;
            let bytes = encode_graph_artifact(&graph)?;
            std::fs::write(path, &bytes)?;
            println!("{}", json!({"graph_digest":graph.metadata().snapshot_digest(),"artifact_sha256":hash(&bytes),"nodes":graph.node_count(),"edges":graph.edge_count(),"generator":"grid-e7/v1","side":side,"spacing_e7":spacing}));
        }
        Some("run" | "unobserved") => {
            let root = Path::new(args.get(2).ok_or("bundle directory")?);
            let output = Path::new(args.get(3).ok_or("fresh output directory")?);
            std::fs::create_dir(output)?;
            let meta: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("bundle.json"))?)?;
            let graph = decode_graph_artifact(&std::fs::read(root.join("graph.rrgraph"))?)?;
            let scenario_bytes = std::fs::read(root.join("scenario.json"))?;
            if meta["manifest_sha256"] != hash(&std::fs::read(root.join("manifest.json"))?)
                || meta["scenario_sha256"] != hash(&scenario_bytes) || meta["graph_digest"] != graph.metadata().snapshot_digest() || meta["artifact_sha256"] != hash(&std::fs::read(root.join("graph.rrgraph"))?) { return Err("bundle identity mismatch".into()); }
            let scenario: SimulationScenario = serde_json::from_slice(&scenario_bytes)?;
            if scenario.graph_snapshot_digest.as_deref() != Some(graph.metadata().snapshot_digest()) { return Err("scenario graph binding missing".into()); }
            let run_identity = format!("phase34:{}",hash(&scenario_bytes));
            let mut progress = BufWriter::new(File::create(output.join("progress.jsonl"))?);
            let began = Instant::now();
            let mut created = 0_u64;
            let mut cancelled = 0_u64;
            let mut last_logical_seconds = None; let mut delivered = 0_u64;
            let mut peak_pending = 0; let mut peak_queue = 0; let mut peak_outstanding = 0;
            let mut entries = 0; let mut io_error = None;

            let mut hist = std::collections::BTreeMap::<String, [u64; 65]>::new();
            let result = if args[1] == "unobserved" {
                simulate_with_run_identity(&graph, &scenario, &run_identity)?
            } else {
                simulate_observed(&graph, &scenario, &run_identity, &mut |p| {
                    let ns = u64::try_from(p.processing_ns).unwrap_or(u64::MAX);
                    hist.entry(p.operation.to_owned()).or_insert([0; 65])[(64 - ns.leading_zeros()) as usize] += 1;
                    for event in p.events {
                        match event.event { SimulationEvent::OrderCreated {..} => created += 1, SimulationEvent::OrderDelivered {..} => delivered += 1, SimulationEvent::DynamicChanged { change: roadrunner_simulation::DynamicChange::Cancel { .. }, applied: true, .. } => cancelled += 1, _ => {} }
                    }
                    entries = p.processed_entries;
                    last_logical_seconds = Some(p.at.value());
                    peak_queue = peak_queue.max(p.queue_depth).max(p.queue_depth_before); peak_pending = peak_pending.max(p.pending_orders);
                    peak_outstanding = peak_outstanding.max(created-delivered-cancelled);
                    if entries == 1 || entries % 100 == 0 {
                        let row = json!({"entries":entries,"logical_seconds":p.at.value(),"elapsed_ns":began.elapsed().as_nanos(),"created":created,"delivered":delivered,"outstanding":created-delivered-cancelled,"pending":p.pending_orders,"riders_with_work":p.riders_with_work,"waiting_riders":p.waiting_riders,"queue_depth":p.queue_depth,"peak_queue":peak_queue,"peak_pending":peak_pending,"peak_outstanding":peak_outstanding,"started_actions":p.started_actions});
                        if let Err(e) = writeln!(progress,"{row}").and_then(|()| progress.flush()) { io_error = Some(e); }
                    }
                })?
            };
            let execution_ns = began.elapsed().as_nanos();
            if let Some(e) = io_error { return Err(e.into()); }
            let s = &result.summary;
            if s.created_orders != s.delivered_orders + s.cancelled_orders + s.outstanding_orders || s.scheduled_orders != s.created_orders + s.uncreated_orders || s.outstanding_orders != s.unassigned_orders + s.assigned_unfinished_orders { return Err("lifecycle accounting mismatch".into()); }
            let serialization = Instant::now();
            let mut writer = EvidenceWriter { file:BufWriter::new(File::create(output.join("result.json"))?), hash:Sha256::new(), bytes:0 };
            serde_json::to_writer(&mut writer,&result)?; writer.flush()?;
            let serialization_ns = serialization.elapsed().as_nanos();
            let histogram: std::collections::BTreeMap<_,_> = hist.into_iter().map(|(key,value)|(key,value.to_vec())).collect();
            let report = json!({"schema_version":1,"scenario_sha256":hash(&scenario_bytes),"graph_digest":graph.metadata().snapshot_digest(),"graph_nodes":graph.node_count(),"graph_edges":graph.edge_count(),"execution_ns":execution_ns,"serialization_ns":serialization_ns,"result_sha256":format!("{:x}",writer.hash.finalize()),"result_bytes":writer.bytes,"summary":s,"configured_riders":scenario.riders.len(),"initial_available_riders":scenario.riders.iter().filter(|r|r.available).count(),"processed_entries":entries,"last_logical_seconds":last_logical_seconds,"peak_pending":peak_pending,"peak_queue":peak_queue,"peak_outstanding":peak_outstanding,"future_events":result.future_events,"event_records":result.events.len(),"starts":result.trace.starts.len(),"history_records":result.trace.history.len(),"queue_action_log2_ns":histogram,"diagnostics":args[1]=="run","limits":"Serial headless simulation; queue action histograms exclude observer but are not HTTP request latencies. Full result retained. Queue depth includes future arrivals; pending and outstanding are separate. No physical telemetry."});
            std::fs::write(output.join("report.json"), serde_json::to_vec_pretty(&report)?)?;
            println!("{report}");
        }
        _ => return Err("usage: phase34_workload grid SIDE SPACING_E7 ARTIFACT | run|unobserved BUNDLE FRESH_OUTPUT".into()),
    }
    Ok(())
}
