"""Generate the deterministic Phase 7 routing-source semantics PBF fixture."""

from pathlib import Path
import sys

import osmium


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: generate_phase7_semantics_fixture.py OUTPUT.osm.pbf")
    output = Path(sys.argv[1])
    output.unlink(missing_ok=True)
    writer = osmium.SimpleWriter(str(output), overwrite=True)
    locations = {
        1: (3.3790, 6.5240),
        2: (3.3791, 6.5240),
        3: (3.3792, 6.5240),
        4: (3.3791, 6.5241),
        5: (3.3793, 6.5240),
    }
    node_tags = {
        2: {"barrier": "gate", "access": "delivery"},
        3: {"ford": "yes", "highway": "ford"},
    }
    for node_id, (longitude, latitude) in locations.items():
        writer.add_node(
            osmium.osm.mutable.Node(
                id=node_id,
                location=osmium.osm.Location(longitude, latitude),
                tags=node_tags.get(node_id, {}),
                version=1,
            )
        )
    writer.add_way(
        osmium.osm.mutable.Way(
            id=100,
            nodes=[1, 2, 3],
            tags={"highway": "residential", "surface": "unpaved"},
            version=1,
        )
    )
    writer.add_way(
        osmium.osm.mutable.Way(
            id=200,
            nodes=[2, 4],
            tags={"highway": "service", "access": "customers"},
            version=1,
        )
    )
    writer.add_way(
        osmium.osm.mutable.Way(
            id=300,
            nodes=[3, 5],
            tags={"route": "ferry", "motor_vehicle": "yes"},
            version=1,
        )
    )
    writer.add_relation(
        osmium.osm.mutable.Relation(
            id=400,
            members=[("w", 100, "from"), ("n", 2, "via"), ("w", 200, "to")],
            tags={
                "type": "restriction",
                "restriction": "no_left_turn",
                "restriction:motorcycle": "only_straight_on",
            },
            version=1,
        )
    )
    writer.add_relation(
        osmium.osm.mutable.Relation(
            id=401,
            members=[("w", 100, "from"), ("n", 3, "via"), ("w", 300, "to")],
            tags={
                "type": "restriction",
                "restriction:motor_vehicle": "unsupported_turn_form",
            },
            version=1,
        )
    )
    writer.close()


if __name__ == "__main__":
    main()
