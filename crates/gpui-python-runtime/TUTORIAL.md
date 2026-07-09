# gpui-python-runtime Tutorial

`gpui-python-runtime` stores retained scene specifications for a Python-facing
GPUI wrapper.

## 1. Add the crate

```toml
[dependencies]
gpui-python-runtime = { workspace = true }
```

Enable GPUI rendering support when building the showcase:

```toml
gpui-python-runtime = { workspace = true, features = ["showcase"] }
```

For Python-authored apps, install the pure-Python declarations from the crate:

```bash
python -m pip install -e crates/gpui-python-runtime
```

The package name is `gpui-toolkit`; import it as `gpui_toolkit`.

## 2. Model retained state

The crate exports:

- `PythonAppIr` for UI-facing app descriptions
- `RetainedSceneCache` for cached resources
- `Scene3D` and related scene types
- `DirtyResources` and `CacheUpdate` for incremental updates

## 3. Update a scene

1. Receive or construct a Python-side IR payload.
2. Validate it into `PythonAppIr`.
3. Update retained resources in `RetainedSceneCache`.
4. Track dirty resources.
5. Render through the GPUI adapter when the `gpui`/`showcase` path is enabled.

## 4. Version JSON payloads

Python app IR and Scene3D specs carry explicit v1 schema versions:

```json
{
  "schema_version": 1,
  "title": "Demo",
  "sections": [{
    "id": "overview",
    "label": "Overview",
    "content": { "kind": "text", "text": "Hello" }
  }]
}
```

Omitted `schema_version` fields are accepted as v1 for compatibility with early
examples. Unsupported future versions are rejected during `PythonAppIr`
validation or Scene3D spec parsing. Keep additive fields optional with Rust
defaults when staying on v1; rename, removal, or semantic changes require a
schema-version bump plus migration tests.

## 5. Run the demo

```bash
cargo run -p gpui-python-runtime --bin gpui-python-showcase --features showcase
```

The demo combines UI-kit components, design tokens, and `gpui-px` charts.

## 6. Verify

```bash
cargo test -p gpui-python-runtime
PYTHONPATH=python python -m unittest discover -s python/tests
python -m pip install -e .
python -m unittest discover -s python/tests
```
