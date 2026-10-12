//! Explicit relational current-fact mapping, not a generic repository abstraction.
use crate::{DurableImage, StorageError, digest};
use postgres::Transaction;
use roadrunner_core::graph::{EdgeId, FrozenGraph, Orientation};
use serde_json::{Map, Value, json};

pub(crate) fn metadata(image: &DurableImage) -> Result<Value, StorageError> {
    let mut value = image.operational.clone();
    let s = value["state"]
        .as_object_mut()
        .ok_or(StorageError::Corrupt)?;
    for key in [
        "plans",
        "execution",
        "contexts",
        "effects",
        "history",
        "publications",
    ] {
        s.remove(key);
    }
    s.get_mut("world")
        .and_then(Value::as_object_mut)
        .ok_or(StorageError::Corrupt)?
        .remove("data");
    Ok(value)
}
pub(crate) fn adapter_metadata(image: &DurableImage) -> Value {
    let mut v = image.adapter.clone();
    if let Some(map) = v.as_object_mut() {
        map.remove("commands");
        map.remove("observations");
    }
    v
}
fn object<'a>(value: &'a Value, key: &str) -> Result<&'a Map<String, Value>, StorageError> {
    value[key].as_object().ok_or(StorageError::Corrupt)
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, StorageError> {
    value[key].as_array().ok_or(StorageError::Corrupt)
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, StorageError> {
    value[key].as_str().ok_or(StorageError::Corrupt)
}
fn number(value: &Value, key: &str) -> Result<f64, StorageError> {
    value[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or(StorageError::Corrupt)
}

// Each normalized table retains one authoritative domain fact. A complete write is
// private in the transaction; SQL constraints and domain validation both apply.
#[allow(clippy::too_many_lines)]
pub(crate) fn write_current(
    tx: &mut Transaction<'_>,
    ns: &str,
    image: &DurableImage,
    graph: &FrozenGraph,
) -> Result<(), StorageError> {
    let s = &image.operational["state"];
    let data = &s["world"]["data"];
    for (id, facts) in object(data, "orders")? {
        let pickup = &facts["pickup"];
        let dropoff = &facts["dropoff"];
        tx.execute("INSERT INTO rr_orders(namespace,id,facts,readiness,fulfillment,pickup,dropoff) VALUES($1,$2::text::rr_u64,$3,$4,$5,ST_SetSRID(ST_MakePoint($6,$7),4326)::geography,ST_SetSRID(ST_MakePoint($8,$9),4326)::geography) ON CONFLICT(namespace,id) DO UPDATE SET facts=excluded.facts,readiness=excluded.readiness,fulfillment=excluded.fulfillment,pickup=excluded.pickup,dropoff=excluded.dropoff",
            &[&ns,&id,&facts,&data["readiness"][id],&data["fulfillment"][id],&number(pickup,"longitude")?,&number(pickup,"latitude")?,&number(dropoff,"longitude")?,&number(dropoff,"latitude")?])?;
    }
    for (id, profile) in object(data, "profiles")? {
        let state = &data["riders"][id];
        let point = &state["coordinate"];
        let revision = s["plans"][id]
            .as_u64()
            .ok_or(StorageError::Corrupt)?
            .to_string();
        tx.execute("INSERT INTO rr_riders(namespace,id,profile,state,plan_revision,location) VALUES($1,$2::text::rr_u64,$3,$4,$5::text::rr_u64,ST_SetSRID(ST_MakePoint($6,$7),4326)::geography) ON CONFLICT(namespace,id) DO UPDATE SET profile=excluded.profile,state=excluded.state,plan_revision=excluded.plan_revision,location=excluded.location",&[&ns,&id,&profile,&state,&revision,&number(point,"longitude")?,&number(point,"latitude")?])?;
    }
    // Current relations change atomically. Stable entity IDs/history are never deleted.
    tx.execute("DELETE FROM rr_assignments WHERE namespace=$1", &[&ns])?;
    for (id, assignment) in object(data, "assignments")? {
        let rider = assignment["rider"]
            .as_u64()
            .ok_or(StorageError::Corrupt)?
            .to_string();
        tx.execute("INSERT INTO rr_assignments(namespace,order_id,rider_id) VALUES($1,$2::text::rr_u64,$3::text::rr_u64)",&[&ns,&id,&rider])?;
    }
    tx.execute("DELETE FROM rr_plan_stops WHERE namespace=$1", &[&ns])?;
    for (rider, plan) in object(data, "plans")? {
        for (ordinal, stop) in array(plan, "stops")?.iter().enumerate() {
            let ordinal = i32::try_from(ordinal).map_err(|_| StorageError::Corrupt)?;
            let (kind, order) = stop
                .as_object()
                .and_then(|o| o.iter().next())
                .ok_or(StorageError::Corrupt)?;
            let order = order.as_u64().ok_or(StorageError::Corrupt)?.to_string();
            tx.execute("INSERT INTO rr_plan_stops(namespace,rider_id,ordinal,order_id,kind) VALUES($1,$2::text::rr_u64,$3,$4::text::rr_u64,$5)",&[&ns,&rider,&ordinal,&order,&kind])?;
        }
    }
    tx.execute("DELETE FROM rr_execution WHERE namespace=$1", &[&ns])?;
    for (rider, body) in object(s, "execution")? {
        let action = body["action_id"]
            .as_u64()
            .ok_or(StorageError::Corrupt)?
            .to_string();
        tx.execute("INSERT INTO rr_execution(namespace,rider_id,action_id,body) VALUES($1,$2::text::rr_u64,$3::text::rr_u64,$4)",&[&ns,&rider,&action,&body])?;
        let route = &body["route"];
        let geometry = geometry(graph, route)?.to_string();
        let graph_digest = text(&route["routing"], "graph_digest")?;
        tx.execute("INSERT INTO rr_execution_routes(namespace,action_id,graph_digest,body,geometry) VALUES($1,$2::text::rr_u64,$3,$4,rr_route_geometry($5)) ON CONFLICT DO NOTHING",&[&ns,&action,&graph_digest,&route,&geometry])?;
    }
    tx.execute("DELETE FROM rr_contexts WHERE namespace=$1", &[&ns])?;
    for (category, body) in object(s, "contexts")? {
        tx.execute(
            "INSERT INTO rr_contexts(namespace,category,body) VALUES($1,$2,$3)",
            &[&ns, &category, &body],
        )?;
    }
    // Recognition includes alias records without a domain revision; immutable append only.
    for (id, body) in object(s, "effects")? {
        let row = tx.query_opt(
            "SELECT body FROM rr_effects WHERE namespace=$1 AND id=$2::text::rr_u64",
            &[&ns, &id],
        )?;
        if row.as_ref().is_some_and(|r| r.get::<_, Value>(0) != *body) {
            return Err(StorageError::Corrupt);
        }
        tx.execute("INSERT INTO rr_effects(namespace,id,body) VALUES($1,$2::text::rr_u64,$3) ON CONFLICT DO NOTHING",&[&ns,&id,&body])?;
    }
    if let Some(traffic) = image.adapter.get("traffic").filter(|v| !v.is_null()) {
        let traffic_digest = text(traffic, "digest")?;
        let graph_digest = text(traffic, "graph_digest")?;
        tx.execute("INSERT INTO rr_traffic(digest,graph_digest,body,sha256) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",&[&traffic_digest,&graph_digest,&traffic,&digest(traffic)])?;
    }
    Ok(())
}
pub(crate) fn write_evidence(
    tx: &mut Transaction<'_>,
    ns: &str,
    image: &DurableImage,
) -> Result<(), StorageError> {
    let s = &image.operational["state"];
    for (id, terms) in object(&s["world"]["data"], "accepted")? {
        let prior = tx.query_opt(
            "SELECT terms FROM rr_deliveries WHERE namespace=$1 AND order_id=$2::text::rr_u64",
            &[&ns, &id],
        )?;
        if prior
            .as_ref()
            .is_some_and(|r| r.get::<_, Value>(0) != *terms)
        {
            return Err(StorageError::Corrupt);
        }
        tx.execute("INSERT INTO rr_deliveries(namespace,order_id,terms) VALUES($1,$2::text::rr_u64,$3) ON CONFLICT DO NOTHING",&[&ns,&id,&terms])?;
    }
    for record in array(s, "history")? {
        let revision = record["after"]["number"]
            .as_u64()
            .ok_or(StorageError::Corrupt)?
            .to_string();
        tx.execute("INSERT INTO rr_history(namespace,revision,body) VALUES($1,$2::text::rr_u64,$3) ON CONFLICT DO NOTHING",&[&ns,&revision,&record])?;
    }
    for record in array(s, "publications")? {
        let revision = record["committed_revision"]["number"]
            .as_u64()
            .ok_or(StorageError::Corrupt)?
            .to_string();
        tx.execute("INSERT INTO rr_publications(namespace,revision,body) VALUES($1,$2::text::rr_u64,$3) ON CONFLICT DO NOTHING",&[&ns,&revision,&record])?;
    }
    for record in array(&image.adapter, "commands")? {
        let principal = text(record, "principal")?;
        let key = text(record, "key")?;
        let body = &record["record"];
        tx.execute("INSERT INTO rr_commands(namespace,principal,key,body) VALUES($1,$2,$3,$4) ON CONFLICT(namespace,principal,key) DO UPDATE SET body=excluded.body WHERE rr_commands.body IS DISTINCT FROM excluded.body",&[&ns,&principal,&key,&body])?;
        if body["command"]["operation"] == "Simulation"
            && body["envelope"]["outcome"] == "completed"
        {
            let command = text(&body["envelope"], "command")?;
            let scenario = &body["command"]["intent"]["scenario"];
            let result = &body["envelope"]["result"];
            tx.execute("INSERT INTO rr_simulations(namespace,command,scenario,result) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",&[&ns,&command,&scenario,&result])?;
        }
    }
    for record in array(&image.adapter, "observations")? {
        let source = text(record, "source")?;
        let id = text(record, "id")?;
        let body = &record["record"];
        let prior = tx.query_opt(
            "SELECT body FROM rr_observations WHERE namespace=$1 AND source=$2 AND id=$3",
            &[&ns, &source, &id],
        )?;
        if prior
            .as_ref()
            .is_some_and(|r| r.get::<_, Value>(0) != *body)
        {
            return Err(StorageError::Corrupt);
        }
        tx.execute("INSERT INTO rr_observations(namespace,source,id,body) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING",&[&ns,&source,&id,&body])?;
    }
    Ok(())
}
#[allow(clippy::too_many_lines)]
pub(crate) fn load(tx: &mut Transaction<'_>, ns: &str) -> Result<DurableImage, StorageError> {
    let row = tx.query_one(
        "SELECT metadata,adapter,time_domain,clock_floor FROM rr_worlds WHERE namespace=$1",
        &[&ns],
    )?;
    let mut operational: Value = row.get(0);
    let mut adapter: Value = row.get(1);
    let mut data = json!({"orders":{},"readiness":{},"fulfillment":{},"profiles":{},"riders":{},"assignments":{},"accepted":{},"plans":{}});
    for row in tx.query(
        "SELECT id::text,facts,readiness,fulfillment FROM rr_orders WHERE namespace=$1 ORDER BY id",
        &[&ns],
    )? {
        let id: String = row.get(0);
        data["orders"][&id] = row.get(1);
        data["readiness"][&id] = row.get(2);
        data["fulfillment"][&id] = row.get(3);
    }
    let s = &mut operational["state"];
    s["plans"] = json!({});
    s["execution"] = json!({});
    s["contexts"] = json!({});
    s["effects"] = json!({});
    for row in tx.query("SELECT id::text,profile,state,plan_revision::text FROM rr_riders WHERE namespace=$1 ORDER BY id",&[&ns])?{let id:String=row.get(0);data["profiles"][&id]=row.get(1);data["riders"][&id]=row.get(2);data["plans"][&id]=json!({"stops":[]});s["plans"][&id]=json!(row.get::<_,String>(3).parse::<u64>().map_err(|_|StorageError::Corrupt)?);}
    for row in tx.query("SELECT order_id::text,rider_id::text FROM rr_assignments WHERE namespace=$1 ORDER BY order_id",&[&ns])?{let id:String=row.get(0);let order=id.parse::<u64>().map_err(|_|StorageError::Corrupt)?;let rider=row.get::<_,String>(1).parse::<u64>().map_err(|_|StorageError::Corrupt)?;data["assignments"][&id]=json!({"order":order,"rider":rider});}
    for row in tx.query(
        "SELECT order_id::text,terms FROM rr_deliveries WHERE namespace=$1 ORDER BY order_id",
        &[&ns],
    )? {
        let id: String = row.get(0);
        data["accepted"][&id] = row.get(1);
    }
    for row in tx.query("SELECT rider_id::text,order_id::text,kind FROM rr_plan_stops WHERE namespace=$1 ORDER BY rider_id,ordinal",&[&ns])?{let rider:String=row.get(0);let order=row.get::<_,String>(1).parse::<u64>().map_err(|_|StorageError::Corrupt)?;let kind:String=row.get(2);data["plans"][&rider]["stops"].as_array_mut().ok_or(StorageError::Corrupt)?.push(json!({kind:order}));}
    for row in tx.query(
        "SELECT rider_id::text,body FROM rr_execution WHERE namespace=$1 ORDER BY rider_id",
        &[&ns],
    )? {
        let id: String = row.get(0);
        s["execution"][&id] = row.get(1);
    }
    for row in tx.query(
        "SELECT category,body FROM rr_contexts WHERE namespace=$1 ORDER BY category",
        &[&ns],
    )? {
        let key: String = row.get(0);
        s["contexts"][&key] = row.get(1);
    }
    for row in tx.query(
        "SELECT id::text,body FROM rr_effects WHERE namespace=$1 ORDER BY id",
        &[&ns],
    )? {
        let id: String = row.get(0);
        s["effects"][&id] = row.get(1);
    }
    s["world"]["data"] = data;
    s["history"] = Value::Array(
        tx.query(
            "SELECT body FROM rr_history WHERE namespace=$1 ORDER BY revision",
            &[&ns],
        )?
        .iter()
        .map(|r| r.get(0))
        .collect(),
    );
    s["publications"] = Value::Array(
        tx.query(
            "SELECT body FROM rr_publications WHERE namespace=$1 ORDER BY revision",
            &[&ns],
        )?
        .iter()
        .map(|r| r.get(0))
        .collect(),
    );
    adapter["commands"]=Value::Array(tx.query("SELECT principal,key,body FROM rr_commands WHERE namespace=$1 ORDER BY principal COLLATE \"C\",key COLLATE \"C\"",&[&ns])?.iter().map(|r|json!({"principal":r.get::<_,String>(0),"key":r.get::<_,String>(1),"record":r.get::<_,Value>(2)})).collect());
    adapter["observations"]=Value::Array(tx.query("SELECT source,id,body FROM rr_observations WHERE namespace=$1 ORDER BY source COLLATE \"C\",id COLLATE \"C\"",&[&ns])?.iter().map(|r|json!({"source":r.get::<_,String>(0),"id":r.get::<_,String>(1),"record":r.get::<_,Value>(2)})).collect());
    Ok(DurableImage {
        operational,
        adapter,
        time_domain: row.get(2),
        clock_floor: row.get(3),
    })
}
fn geometry(graph: &FrozenGraph, route: &Value) -> Result<Value, StorageError> {
    let mut points = Vec::new();
    for edge in array(route, "edges")? {
        let id = u32::try_from(edge.as_u64().ok_or(StorageError::Corrupt)?)
            .map_err(|_| StorageError::Corrupt)?;
        let edge = graph.edge(EdgeId::new(id)).ok_or(StorageError::Corrupt)?;
        let geometry = graph
            .segment_geometry(edge.segment())
            .map_err(|_| StorageError::Corrupt)?;
        let ordered: Vec<_> = if edge.orientation() == Orientation::Reverse {
            geometry.iter().rev().copied().collect()
        } else {
            geometry.to_vec()
        };
        for point in ordered {
            let p = point.to_coordinate();
            let p = [p.longitude(), p.latitude()];
            if points.last() != Some(&p) {
                points.push(p);
            }
        }
    }
    if points.is_empty() {
        let id = u32::try_from(route["from"].as_u64().ok_or(StorageError::Corrupt)?)
            .map_err(|_| StorageError::Corrupt)?;
        let c = graph
            .node(roadrunner_core::graph::NodeId::new(id))
            .ok_or(StorageError::Corrupt)?
            .coordinate();
        points.push([c.longitude(), c.latitude()]);
    }
    if points.len() == 1 {
        points.push(points[0]);
    }
    Ok(json!({"type":"LineString","coordinates":points}))
}
