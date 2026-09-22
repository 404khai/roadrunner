"""Convert an OSM XML fixture to a deterministic PBF fixture."""

from pathlib import Path
import sys

import osmium


class Copier(osmium.SimpleHandler):
    def __init__(self, writer: osmium.SimpleWriter) -> None:
        super().__init__()
        self.writer = writer

    def node(self, node: osmium.osm.Node) -> None:
        self.writer.add_node(node)

    def way(self, way: osmium.osm.Way) -> None:
        self.writer.add_way(way)

    def relation(self, relation: osmium.osm.Relation) -> None:
        self.writer.add_relation(relation)


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: convert_osm_to_pbf.py INPUT.osm OUTPUT.osm.pbf")
    source = Path(sys.argv[1])
    output = Path(sys.argv[2])
    output.unlink(missing_ok=True)
    writer = osmium.SimpleWriter(str(output), overwrite=True)
    copier = Copier(writer)
    copier.apply_file(str(source), locations=False)
    writer.close()


if __name__ == "__main__":
    main()
