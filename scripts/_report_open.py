import collections
import json
import pathlib
import tomllib

root = pathlib.Path(__file__).resolve().parents[1]
inventory = json.loads((root / "python-rustdoc-inventory.json").read_text())
symbols = {
    f"{symbol['package']}:{symbol['id']}": symbol
    for crate in inventory["crates"]
    for symbol in crate["symbols"]
}
data = tomllib.loads((root / "python-surface.toml").read_text())
classified = set()
for entry in data.get("rustdoc_classification", []):
    key = f"{entry.get('package', '')}:{entry.get('rustdoc_id', '')}"
    if key in symbols:
        classified.add(key)
    if entry.get("include_signature"):
        classified.update(
            child_key
            for child_key, child in symbols.items()
            if child.get("owner") == entry.get("rustdoc_id")
            and child.get("package") == entry.get("package")
            and child.get("kind") in {"parameter", "return"}
        )
for rule in data.get("rustdoc_classification_rule", []):
    prefix = f"{rule.get('package', '')}:{rule.get('path_prefix', '')}"
    classified.update(
        key
        for key in symbols
        if key == prefix or key.startswith(prefix + "::") or key.startswith(prefix + "#")
    )
by_area = collections.Counter()
for key, symbol in symbols.items():
    if key in classified:
        continue
    package, rust_id = key.split(":", 1)
    parts = rust_id.split("::")
    area = "::".join(parts[:3]) if len(parts) >= 3 else rust_id
    by_area[(package, area)] += 1
for (package, area), count in by_area.most_common():
    print(f"{count:4} {package}:{area}")
