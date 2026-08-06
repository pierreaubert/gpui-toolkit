# Bundled native host

Release packaging places `gpui-python-host` (or `.exe`) in this directory via
`scripts/build-python-wheel.sh`. It is intentionally absent from source control:
the executable is target-platform specific.
