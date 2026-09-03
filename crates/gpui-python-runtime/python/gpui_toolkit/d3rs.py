"""Public v2 module for typed :mod:`gpui-d3rs` operations.

The older ``gpui_toolkit.d3`` spelling remains a source-compatible shim. New
applications should import this module so the Python package topology matches
the Rust library name used by the capability registry.
"""

from .d3 import *  # noqa: F403
