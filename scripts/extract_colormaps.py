#!/usr/bin/env python3
"""Extract the built-in colormap tables from the C++ Polyscope source.

`src/render/color_maps.cpp` holds each colormap as a `std::vector<glm::vec3>`
with the name `CM_<NAME>`. This script reads those tables and writes one
binary file for each colormap to the polyscope-rs data directory.

File format: RGB triples. Each component is a little-endian 32-bit float.
The number of colors is the file size divided by 12.

Usage:
    python3 scripts/extract_colormaps.py

Assumes:
    - C++ source at ~/repo/polyscope/src/render/color_maps.cpp
    - Output to  crates/polyscope-render/data/colormaps/
"""

import os
import re
import struct
import sys

SOURCE_FILE = os.path.expanduser("~/repo/polyscope/src/render/color_maps.cpp")
OUTPUT_DIR = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "crates", "polyscope-render", "data", "colormaps",
)

# Maps the C++ table name to the colormap name that users see.
# The names and their order match `Engine::loadDefaultColorMaps()`.
COLORMAPS = [
    ("CM_VIRIDIS", "viridis"),
    ("CM_PLASMA", "plasma"),
    ("CM_INFERNO", "inferno"),
    ("CM_MAGMA", "magma"),
    ("CM_GRAY", "gray"),
    ("CM_COOLWARM", "coolwarm"),
    ("CM_BLUES", "blues"),
    ("CM_REDS", "reds"),
    ("CM_PIYG", "pink-green"),
    ("CM_PHASE", "phase"),
    ("CM_SPECTRAL", "spectral"),
    ("CM_RAINBOW", "rainbow"),
    ("CM_JET", "jet"),
    ("CM_TURBO", "turbo"),
    ("CM_HSV", "hsv"),
]

TABLE_PATTERN = r"const std::vector<glm::vec3> {name}\s*=\s*\{{(.*?)\}};"
TRIPLE_PATTERN = re.compile(r"\{([^{}]*)\}")


def extract_table(source: str, table_name: str) -> list[tuple[float, float, float]]:
    """Return the RGB triples of one C++ colormap table."""
    match = re.search(TABLE_PATTERN.format(name=table_name), source, re.DOTALL)
    if match is None:
        raise ValueError(f"table {table_name} not found")
    colors = []
    for triple in TRIPLE_PATTERN.findall(match.group(1)):
        components = [float(value) for value in triple.split(",")]
        if len(components) != 3:
            raise ValueError(f"{table_name}: expected 3 components, found {len(components)}")
        colors.append((components[0], components[1], components[2]))
    if len(colors) < 2:
        raise ValueError(f"{table_name}: expected 2 or more colors, found {len(colors)}")
    return colors


def main() -> int:
    with open(SOURCE_FILE, encoding="utf-8") as source_file:
        source = source_file.read()

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    for table_name, colormap_name in COLORMAPS:
        colors = extract_table(source, table_name)
        output_path = os.path.join(OUTPUT_DIR, f"{colormap_name}.bin")
        with open(output_path, "wb") as output_file:
            output_file.writelines(struct.pack("<3f", *color) for color in colors)
        print(f"{colormap_name}: {len(colors)} colors -> {output_path}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
