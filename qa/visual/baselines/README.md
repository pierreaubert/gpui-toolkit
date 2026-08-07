# Renderer baseline archives

Visual baselines are stored as versioned compressed archives instead of
hundreds of loose PNG files. Each archive expands beneath the visual output
root and contains a renderer namespace, validated PNGs, and `index.json` /
`index.md` checksum evidence.

`component-lab-metal-pr-v1.tar.zst` contains the deterministic 200-case PR
profile for the Metal renderer at a 2x device-pixel scale. The selection keeps
at least one rendered case for every registered component story and rotates
through viewport and design presets.

The normal QA script extracts the archive automatically. Intentional baseline
changes require an explicit local approval run:

```bash
QA_VISUAL_UPDATE_BASELINES=1 QA_VISUAL_CAPTURE_LIMIT=200 \
  scripts/qa_visual_capture.sh
```

After reviewing the generated actual images, diffs, capture report, and contact
sheets, rebuild the archive from the visual output root:

```bash
tar -C target/qa/visual/component-lab -cf - metal/baseline \
  | zstd -19 -T0 -o qa/visual/baselines/component-lab-metal-pr-v1.tar.zst
```

Do not promote baselines merely to make CI green. Baseline changes are review
artifacts and must be explained alongside the UI change that produced them.
