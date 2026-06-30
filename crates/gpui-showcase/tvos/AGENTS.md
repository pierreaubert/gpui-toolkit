# gpui-showcase-tvos

tvOS showcase app wrapper for `gpui-showcase`.

## Architecture

Static library (`crate-type = ["staticlib"]`) linked into a Swift tvOS app.
The Swift AppDelegate calls `showcase_tvos_start()` and drives GPUI frames via
`CADisplayLink`.

## Testing

```bash
cargo +nightly build -p gpui-showcase-tvos --target aarch64-apple-tvos-sim --release -Zbuild-std
just tvos-sim
```
