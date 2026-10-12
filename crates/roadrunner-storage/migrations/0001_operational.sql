-- Offline, checksum-pinned schema 1. No destructive/down migration is provided.
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE DOMAIN rr_u64 AS numeric(20,0) CHECK (VALUE >= 0 AND VALUE <= 18446744073709551615);
CREATE TABLE rr_installation (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    cluster_id text NOT NULL, database_oid bigint NOT NULL, authority_table_oid bigint NOT NULL,
    schema_version integer NOT NULL CHECK (schema_version = 1), migration_sha256 text NOT NULL
);
CREATE TABLE rr_graphs (digest text PRIMARY KEY, sha256 text NOT NULL, artifact bytea NOT NULL);
CREATE TABLE rr_worlds (
    namespace text PRIMARY KEY, world_key text UNIQUE NOT NULL, fence bigint NOT NULL CHECK(fence>0),
    storage_revision bigint NOT NULL CHECK(storage_revision>=0), operational_revision rr_u64 NOT NULL,
    content_sha256 text NOT NULL, graph_digest text NOT NULL REFERENCES rr_graphs(digest),
    config_digest text NOT NULL, time_domain text NOT NULL, clock_floor double precision NOT NULL CHECK(clock_floor>=0 AND clock_floor<'Infinity'),
    metadata jsonb NOT NULL, adapter jsonb NOT NULL,
    certified_until double precision, gate_at double precision NOT NULL CHECK(gate_at>=0),
    CHECK(operational_revision=(metadata#>>'{state,revision,number}')::numeric)
);
CREATE TABLE rr_commits (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), revision bigint NOT NULL CHECK(revision>=0),
    content_sha256 text NOT NULL, previous_sha256 text, PRIMARY KEY(namespace,revision)
);
CREATE TABLE rr_orders (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), id rr_u64 NOT NULL,
    facts jsonb NOT NULL, readiness jsonb NOT NULL, fulfillment jsonb NOT NULL,
    pickup geography(Point,4326) NOT NULL, dropoff geography(Point,4326) NOT NULL,
    PRIMARY KEY(namespace,id), CHECK ((facts->>'id')::numeric=id),
    CHECK(ST_X(pickup::geometry)=(facts#>>'{pickup,longitude}')::double precision AND ST_Y(pickup::geometry)=(facts#>>'{pickup,latitude}')::double precision),
    CHECK(ST_X(dropoff::geometry)=(facts#>>'{dropoff,longitude}')::double precision AND ST_Y(dropoff::geometry)=(facts#>>'{dropoff,latitude}')::double precision)
);
CREATE INDEX rr_orders_pickup_gist ON rr_orders USING gist(pickup);
CREATE TABLE rr_riders (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), id rr_u64 NOT NULL,
    profile jsonb NOT NULL, state jsonb NOT NULL, plan_revision rr_u64 NOT NULL,
    location geography(Point,4326) NOT NULL, PRIMARY KEY(namespace,id), CHECK((profile->>'id')::numeric=id),
    CHECK(ST_X(location::geometry)=(state#>>'{coordinate,longitude}')::double precision AND ST_Y(location::geometry)=(state#>>'{coordinate,latitude}')::double precision)
);
CREATE INDEX rr_riders_location_gist ON rr_riders USING gist(location);
CREATE TABLE rr_assignments (
    namespace text NOT NULL, order_id rr_u64 NOT NULL, rider_id rr_u64 NOT NULL,
    PRIMARY KEY(namespace,order_id), FOREIGN KEY(namespace,order_id) REFERENCES rr_orders(namespace,id),
    FOREIGN KEY(namespace,rider_id) REFERENCES rr_riders(namespace,id)
);
-- Acceptance terms are the delivery commitment; custody/fulfillment lives in rr_orders.
CREATE TABLE rr_deliveries (
    namespace text NOT NULL, order_id rr_u64 NOT NULL, terms jsonb NOT NULL,
    PRIMARY KEY(namespace,order_id), FOREIGN KEY(namespace,order_id) REFERENCES rr_orders(namespace,id)
);
CREATE TABLE rr_plan_stops (
    namespace text NOT NULL, rider_id rr_u64 NOT NULL, ordinal integer NOT NULL CHECK(ordinal>=0),
    order_id rr_u64 NOT NULL, kind text NOT NULL CHECK(kind IN ('Pickup','Dropoff')),
    PRIMARY KEY(namespace,rider_id,ordinal), FOREIGN KEY(namespace,rider_id) REFERENCES rr_riders(namespace,id),
    FOREIGN KEY(namespace,order_id) REFERENCES rr_orders(namespace,id)
);
CREATE TABLE rr_execution (
    namespace text NOT NULL, rider_id rr_u64 NOT NULL, action_id rr_u64 NOT NULL, body jsonb NOT NULL,
    PRIMARY KEY(namespace,rider_id), UNIQUE(namespace,action_id), FOREIGN KEY(namespace,rider_id) REFERENCES rr_riders(namespace,id)
);
CREATE TABLE rr_contexts (namespace text NOT NULL REFERENCES rr_worlds(namespace), category text NOT NULL, body jsonb NOT NULL, PRIMARY KEY(namespace,category));
CREATE TABLE rr_effects (namespace text NOT NULL REFERENCES rr_worlds(namespace), id rr_u64 NOT NULL, body jsonb NOT NULL, PRIMARY KEY(namespace,id), CHECK((body->>'id')::numeric=id));
CREATE TABLE rr_history (namespace text NOT NULL REFERENCES rr_worlds(namespace), revision rr_u64 NOT NULL, body jsonb NOT NULL, PRIMARY KEY(namespace,revision));
CREATE TABLE rr_publications (namespace text NOT NULL REFERENCES rr_worlds(namespace), revision rr_u64 NOT NULL, body jsonb NOT NULL, PRIMARY KEY(namespace,revision));
CREATE TABLE rr_commands (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), principal text NOT NULL, key text NOT NULL,
    body jsonb NOT NULL, PRIMARY KEY(namespace,principal,key)
);
CREATE TABLE rr_observations (namespace text NOT NULL REFERENCES rr_worlds(namespace), source text NOT NULL, id text NOT NULL, body jsonb NOT NULL, PRIMARY KEY(namespace,source,id));
CREATE TABLE rr_traffic (digest text PRIMARY KEY, graph_digest text NOT NULL REFERENCES rr_graphs(digest), body jsonb NOT NULL, sha256 text NOT NULL);
CREATE TABLE rr_routes (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), storage_revision bigint NOT NULL, ordinal integer NOT NULL,
    graph_digest text NOT NULL REFERENCES rr_graphs(digest), request jsonb NOT NULL, response jsonb NOT NULL,
    geometry geometry(Geometry,4326) NOT NULL CHECK(GeometryType(geometry) IN ('LINESTRING','POINT') AND ST_IsValid(geometry)), PRIMARY KEY(namespace,storage_revision,ordinal)
);
CREATE INDEX rr_routes_geometry_gist ON rr_routes USING gist(geometry);
CREATE TABLE rr_execution_routes (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), action_id rr_u64 NOT NULL, graph_digest text NOT NULL REFERENCES rr_graphs(digest),
    body jsonb NOT NULL, geometry geometry(Geometry,4326) NOT NULL CHECK(GeometryType(geometry) IN ('LINESTRING','POINT') AND ST_IsValid(geometry)), PRIMARY KEY(namespace,action_id)
);
CREATE TABLE rr_simulations (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), command text NOT NULL, scenario jsonb NOT NULL,
    result jsonb NOT NULL, PRIMARY KEY(namespace,command)
);
CREATE TABLE rr_outbox (
    namespace text NOT NULL REFERENCES rr_worlds(namespace), storage_revision bigint NOT NULL,
    ordinal integer NOT NULL CHECK(ordinal>=0), event_id text UNIQUE NOT NULL, body jsonb NOT NULL,
    PRIMARY KEY(namespace,storage_revision,ordinal)
);
CREATE FUNCTION rr_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'immutable Roadrunner evidence'; END $$;
CREATE FUNCTION rr_command_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.body->'intent' IS DISTINCT FROM NEW.body->'intent'
       OR (OLD.body#>>'{envelope,outcome}' <> 'pending' AND OLD.body IS DISTINCT FROM NEW.body)
    THEN RAISE EXCEPTION 'immutable command intent or terminal result'; END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER rr_command_guard BEFORE UPDATE ON rr_commands FOR EACH ROW EXECUTE FUNCTION rr_command_guard();
CREATE FUNCTION rr_commit_gate() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE at_seconds double precision := extract(epoch FROM clock_timestamp());
BEGIN
    IF at_seconds < NEW.clock_floor THEN RAISE EXCEPTION 'Roadrunner database clock regressed behind operational gate'; END IF;
    IF NEW.certified_until IS NOT NULL AND at_seconds > NEW.certified_until
    THEN RAISE EXCEPTION 'Roadrunner temporal certificate expired at storage commit gate'; END IF;
    RETURN NEW;
END $$;
-- Last transaction gate runs at COMMIT, not only at an earlier application clock sample.
CREATE CONSTRAINT TRIGGER rr_commit_gate AFTER UPDATE ON rr_worlds DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION rr_commit_gate();
CREATE TRIGGER rr_graphs_immutable BEFORE UPDATE OR DELETE ON rr_graphs FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_history_immutable BEFORE UPDATE OR DELETE ON rr_history FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_publications_immutable BEFORE UPDATE OR DELETE ON rr_publications FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_observations_immutable BEFORE UPDATE OR DELETE ON rr_observations FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_traffic_immutable BEFORE UPDATE OR DELETE ON rr_traffic FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_routes_immutable BEFORE UPDATE OR DELETE ON rr_routes FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_execution_routes_immutable BEFORE UPDATE OR DELETE ON rr_execution_routes FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_simulations_immutable BEFORE UPDATE OR DELETE ON rr_simulations FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_outbox_immutable BEFORE UPDATE OR DELETE ON rr_outbox FOR EACH ROW EXECUTE FUNCTION rr_immutable();

CREATE TRIGGER rr_deliveries_immutable BEFORE UPDATE OR DELETE ON rr_deliveries FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_effects_immutable BEFORE UPDATE OR DELETE ON rr_effects FOR EACH ROW EXECUTE FUNCTION rr_immutable();

-- Zero-length wire LineStrings retain their response but index as a valid point.
CREATE FUNCTION rr_route_geometry(source text) RETURNS geometry LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE g geometry := ST_SetSRID(ST_GeomFromGeoJSON(source),4326);
BEGIN IF ST_Length(g)=0 THEN RETURN ST_StartPoint(g); ELSE RETURN g; END IF; END $$;

CREATE TRIGGER rr_commits_immutable BEFORE UPDATE OR DELETE ON rr_commits FOR EACH ROW EXECUTE FUNCTION rr_immutable();
