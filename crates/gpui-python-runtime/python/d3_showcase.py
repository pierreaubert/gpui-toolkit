"""Source-tree entry point for the installed Python-authored d3rs showcase."""

from gpui_toolkit.d3_showcase import D3RS_SECTION_ORDER, D3rsShowcase, build_app, main

__all__ = ["D3RS_SECTION_ORDER", "D3rsShowcase", "build_app", "main"]


if __name__ == "__main__":
    main()
