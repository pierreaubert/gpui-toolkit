# MeshPlot Metal reference evidence

- source revision: `82c91b46cfcd7f8367bb3d86bfe6517cd7c74f44`
- source tree: dirty during development capture; promote again from a clean release commit
- renderer: macOS Metal, 2× pixel scale
- local capture manifest: `target/qa/visual/component-lab-capture.json`
- local actual captures: 99 requested, 99 captured, 0 failed
- baseline archive: `qa/visual/baselines/component-lab-metal-pr-v1.tar.zst`
- diff report: `target/qa/visual/component-lab-diff.json`
- versioned baseline captures: 9
- cases: 9 compared, 0 failed
- visual threshold: 0 changed pixels

The 99 local actual captures are the complete component-lab story × viewport ×
scheme matrix. The versioned archive intentionally contains only the 9 reviewed
`px-mesh-plot` reference captures listed below; the release-evidence gate checks
both counts and requires the diff report IDs to match the versioned archive.

| Capture | Dimensions | Changed pixels | Max channel delta |
| --- | ---: | ---: | ---: |
| `px-mesh-plot__dashboard-wide__dark` | 2560×1520 | 0 | 0 |
| `px-mesh-plot__dashboard-wide__high-contrast` | 2560×1520 | 0 | 0 |
| `px-mesh-plot__dashboard-wide__light` | 2560×1520 | 0 | 0 |
| `px-mesh-plot__mobile-card__dark` | 780×1280 | 0 | 0 |
| `px-mesh-plot__mobile-card__high-contrast` | 780×1280 | 0 | 0 |
| `px-mesh-plot__mobile-card__light` | 780×1280 | 0 | 0 |
| `px-mesh-plot__panel-compact__dark` | 1440×1040 | 0 | 0 |
| `px-mesh-plot__panel-compact__high-contrast` | 1440×1040 | 0 | 0 |
| `px-mesh-plot__panel-compact__light` | 1440×1040 | 0 | 0 |

The baseline members were promoted from the corresponding successful Metal
captures and then compared by component-lab. No synthetic pixels or inferred
visual results are included.
