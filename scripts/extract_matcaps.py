#!/usr/bin/env python3
"""Extract matcap HDR files from C++ Polyscope bindata .cpp files.

Each bindata_*.cpp contains one or more `std::array<unsigned char, N>` with
hex-encoded RADIANCE HDR data. This script parses those arrays and writes
the raw bytes to .hdr files in the polyscope-rs data directory.

Usage:
    python3 scripts/extract_matcaps.py

Assumes:
    - C++ source at ~/repo/polyscope/src/render/bindata/
    - Output to  crates/polyscope-render/data/matcaps/
"""

import os
import re
import sys

BINDATA_DIR = os.path.expanduser("~/repo/polyscope/src/render/bindata")
OUTPUT_DIR = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "crates", "polyscope-render", "data", "matcaps",
)

# Maps bindata file -> list of (array_name_suffix, output_filename)
MATERIALS = {
    # Blendable materials (4 channels each)
    "bindata_clay.cpp":    [("_r", "clay_r.hdr"), ("_g", "clay_g.hdr"), ("_b", "clay_b.hdr"), ("_k", "clay_k.hdr")],
    "bindata_wax.cpp":     [("_r", "wax_r.hdr"),  ("_g", "wax_g.hdr"),  ("_b", "wax_b.hdr"),  ("_k", "wax_k.hdr")],
    "bindata_candy.cpp":   [("_r", "candy_r.hdr"), ("_g", "candy_g.hdr"), ("_b", "candy_b.hdr"), ("_k", "candy_k.hdr")],
    "bindata_flat.cpp":    [("_r", "flat_r.hdr"),  ("_g", "flat_g.hdr"),  ("_b", "flat_b.hdr"),  ("_k", "flat_k.hdr")],
    # Static materials (1 channel, reused for all 4)
    "bindata_mud.cpp":     [("", "mud.hdr")],
    "bindata_ceramic.cpp": [("", "ceramic.hdr")],
    "bindata_jade.cpp":    [("", "jade.hdr")],
    "bindata_normal.cpp":  [("", "normal.hdr")],
}


def extract_arrays(filepath):
    """Parse a bindata .cpp file and extract all byte arrays.

    Returns dict mapping array name -> bytes.
    """
    with open(filepath, "r") as f:
        content = f.read()

    # Pattern: const std::array<unsigned char, N> NAME = { ... };
    pattern = r'const\s+std::array<unsigned\s+char,\s*\d+>\s+(\w+)\s*=\s*\{([^}]+)\}'
    results = {}

    for match in re.finditer(pattern, content, re.DOTALL):
        name = match.group(1)
        hex_str = match.group(2)
        # Extract all hex values
        hex_values = re.findall(r'0x([0-9a-fA-F]{2})', hex_str)
        data = bytes(int(h, 16) for h in hex_values)
        results[name] = data

    return results


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    total_files = 0
    total_bytes = 0

    for cpp_file, channels in MATERIALS.items():
        filepath = os.path.join(BINDATA_DIR, cpp_file)
        if not os.path.exists(filepath):
            print(f"WARNING: {filepath} not found, skipping")
            continue

        print(f"Processing {cpp_file}...")
        arrays = extract_arrays(filepath)

        for suffix, out_name in channels:
            # Find the matching array
            matching = [name for name in arrays if name.endswith(suffix)]
            if not matching:
                # For static materials, the array name might be just "bindata_<mat>"
                matching = list(arrays.keys())

            if not matching:
                print(f"  WARNING: No array found for suffix '{suffix}' in {cpp_file}")
                continue

            # Use the first match (for single-channel materials) or exact suffix match
            if suffix:
                array_name = [n for n in matching if n.endswith(suffix)][0]
            else:
                array_name = matching[0]

            data = arrays[array_name]
            out_path = os.path.join(OUTPUT_DIR, out_name)

            with open(out_path, "wb") as f:
                f.write(data)

            total_files += 1
            total_bytes += len(data)
            print(f"  {array_name} -> {out_name} ({len(data)} bytes)")

    print(f"\nDone: {total_files} files, {total_bytes:,} bytes total")
    print(f"Output directory: {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
