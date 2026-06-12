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

## 4. Run the demo

```bash
cargo run -p gpui-python-runtime --bin gpui-python-showcase --features showcase
```

The demo combines UI-kit components, design tokens, and `gpui-px` charts.

## 5. Verify

```bash
cargo test -p gpui-python-runtime
```
