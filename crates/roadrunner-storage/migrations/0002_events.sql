-- Additive delivery schema; original operational/anchor formats remain unchanged.
CREATE TABLE rr_event_schema(singleton boolean PRIMARY KEY CHECK(singleton), version integer NOT NULL CHECK(version=1), checksum text NOT NULL);
CREATE TABLE rr_stream_heads(namespace text PRIMARY KEY REFERENCES rr_worlds(namespace), sequence bigint NOT NULL CHECK(sequence>=0), event_id text);
CREATE TABLE rr_stream (
 namespace text NOT NULL REFERENCES rr_worlds(namespace), sequence bigint NOT NULL CHECK(sequence>0),
 event_id text UNIQUE NOT NULL REFERENCES rr_outbox(event_id), predecessor text REFERENCES rr_stream(event_id),
 body jsonb NOT NULL, fingerprint text NOT NULL, PRIMARY KEY(namespace,sequence)
);
CREATE TABLE rr_publish_ack(event_id text PRIMARY KEY REFERENCES rr_stream(event_id), fingerprint text NOT NULL, at timestamptz NOT NULL DEFAULT clock_timestamp());
CREATE TABLE rr_event_consumers(namespace text NOT NULL REFERENCES rr_worlds(namespace), consumer text NOT NULL, semantics text NOT NULL, kind text NOT NULL CHECK(kind IN ('projection','notification')), sequence bigint NOT NULL DEFAULT 0 CHECK(sequence>=0), PRIMARY KEY(namespace,consumer));
CREATE TABLE rr_inbox (
 namespace text NOT NULL, consumer text NOT NULL, source text NOT NULL, message_id text NOT NULL,
 sequence bigint NOT NULL CHECK(sequence>0), event_id text NOT NULL, fingerprint text NOT NULL, payload jsonb NOT NULL,
 state text NOT NULL CHECK(state IN ('deferred','quarantined','applied','already_applied','skipped')),
 outcome jsonb NOT NULL, PRIMARY KEY(namespace,consumer,source,message_id),
 FOREIGN KEY(namespace,consumer) REFERENCES rr_event_consumers(namespace,consumer)
);
CREATE TABLE rr_consumer_effects (
 namespace text NOT NULL, consumer text NOT NULL, event_id text NOT NULL, sequence bigint NOT NULL, fingerprint text NOT NULL, effect jsonb NOT NULL,
 PRIMARY KEY(namespace,consumer,event_id), FOREIGN KEY(namespace,consumer) REFERENCES rr_event_consumers(namespace,consumer)
);
CREATE TABLE rr_projection (
 namespace text NOT NULL, consumer text NOT NULL, path text NOT NULL, value jsonb NOT NULL,
 PRIMARY KEY(namespace,consumer,path), FOREIGN KEY(namespace,consumer) REFERENCES rr_event_consumers(namespace,consumer)
);
-- This is a local notification obligation, never a claim of remote notification delivery.
CREATE TABLE rr_notifications (
 namespace text NOT NULL, consumer text NOT NULL, event_id text NOT NULL, body jsonb NOT NULL,
 PRIMARY KEY(namespace,consumer,event_id), FOREIGN KEY(namespace,consumer,event_id) REFERENCES rr_consumer_effects(namespace,consumer,event_id)
);
CREATE TABLE rr_delivery_ack (
 namespace text NOT NULL, consumer text NOT NULL, source text NOT NULL, message_id text NOT NULL, fingerprint text NOT NULL,
 at timestamptz NOT NULL DEFAULT clock_timestamp(), PRIMARY KEY(namespace,consumer,source,message_id),
 FOREIGN KEY(namespace,consumer,source,message_id) REFERENCES rr_inbox(namespace,consumer,source,message_id)
);
CREATE TABLE rr_event_attempts (
 id bigserial PRIMARY KEY, namespace text NOT NULL REFERENCES rr_worlds(namespace), actor text NOT NULL,
 message_id text NOT NULL, fingerprint text NOT NULL, payload jsonb NOT NULL, result text NOT NULL,
 at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE rr_reconciliations (
 id bigserial PRIMARY KEY, namespace text NOT NULL, consumer text NOT NULL, source text NOT NULL, message_id text NOT NULL,
 principal text NOT NULL, reason text NOT NULL, expected_fingerprint text NOT NULL, resolution text NOT NULL CHECK(resolution='skip'),
 at timestamptz NOT NULL DEFAULT clock_timestamp(),
 FOREIGN KEY(namespace,consumer,source,message_id) REFERENCES rr_inbox(namespace,consumer,source,message_id)
);
CREATE FUNCTION rr_inbox_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.namespace IS DISTINCT FROM NEW.namespace OR OLD.consumer IS DISTINCT FROM NEW.consumer OR OLD.source IS DISTINCT FROM NEW.source
 OR OLD.message_id IS DISTINCT FROM NEW.message_id OR OLD.sequence IS DISTINCT FROM NEW.sequence OR OLD.event_id IS DISTINCT FROM NEW.event_id
 OR OLD.fingerprint IS DISTINCT FROM NEW.fingerprint OR OLD.payload IS DISTINCT FROM NEW.payload
 OR (OLD.state IN ('applied','already_applied','skipped') AND OLD IS DISTINCT FROM NEW)
 THEN RAISE EXCEPTION 'immutable inbox identity or terminal outcome'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER rr_inbox_guard BEFORE UPDATE ON rr_inbox FOR EACH ROW EXECUTE FUNCTION rr_inbox_guard();
CREATE TRIGGER rr_stream_immutable BEFORE UPDATE OR DELETE ON rr_stream FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_publish_ack_immutable BEFORE UPDATE OR DELETE ON rr_publish_ack FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_consumer_effects_immutable BEFORE UPDATE OR DELETE ON rr_consumer_effects FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_notifications_immutable BEFORE UPDATE OR DELETE ON rr_notifications FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_delivery_ack_immutable BEFORE UPDATE OR DELETE ON rr_delivery_ack FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_event_attempts_immutable BEFORE UPDATE OR DELETE ON rr_event_attempts FOR EACH ROW EXECUTE FUNCTION rr_immutable();
CREATE TRIGGER rr_reconciliations_immutable BEFORE UPDATE OR DELETE ON rr_reconciliations FOR EACH ROW EXECUTE FUNCTION rr_immutable();
-- Independent acknowledgement anchors bind these role-specific transaction receipts.
CREATE TABLE rr_event_heads(namespace text NOT NULL REFERENCES rr_worlds(namespace), actor text NOT NULL, revision bigint NOT NULL CHECK(revision>=0), fingerprint text NOT NULL, PRIMARY KEY(namespace,actor));
CREATE TABLE rr_event_receipts(namespace text NOT NULL, actor text NOT NULL, revision bigint NOT NULL CHECK(revision>=0), fingerprint text NOT NULL, previous text, PRIMARY KEY(namespace,actor,revision), FOREIGN KEY(namespace,actor) REFERENCES rr_event_heads(namespace,actor));
CREATE TRIGGER rr_event_receipts_immutable BEFORE UPDATE OR DELETE ON rr_event_receipts FOR EACH ROW EXECUTE FUNCTION rr_immutable();
