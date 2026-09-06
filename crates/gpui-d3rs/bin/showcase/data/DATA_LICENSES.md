# Showcase data provenance

`land-50m.json` is the `world-atlas@2/land-50m.json` TopoJSON artifact,
downloaded from:

<https://cdn.jsdelivr.net/npm/world-atlas@2/land-50m.json>

SHA-256:
`619477ff690c086885e45cb91707d783805561bd75ae8e437b7d4694b0204e0f`.

World Atlas is distributed under the ISC license and derives this geography
from [Natural Earth](https://www.naturalearthdata.com/about/terms-of-use/),
whose map data is in the public domain. The file is tracked because library
tests and the showcase embed it at compile time; the adjacent regeneration
script verifies the digest before replacing it.

`counties-albers-10m.json` is the `us-atlas@3/counties-albers-10m.json`
TopoJSON artifact (U.S. counties, pre-projected with Albers USA), downloaded
from:

<https://cdn.jsdelivr.net/npm/us-atlas@3/counties-albers-10m.json>

SHA-256:
`a674dfa31b625e92635f32684a61ae135b05a91507f17f2d5164833237fecf46`.

US Atlas is distributed under the ISC license and derives this geography
from the U.S. Census Bureau, whose map data is in the public domain. It is
joined in the choropleth example with the in-repo `unemployment-x.csv`
(county unemployment rates by FIPS id).
