const d3 = require('d3');
const fs = require('fs');
const path = require('path');
const topojson = require('topojson-client');

// Load D3's internal clip modules directly from node_modules.
const clip = require('../node_modules/d3-geo/src/clip/index.js').default;
const clipCircle = require('../node_modules/d3-geo/src/clip/circle.js').default;

const DATA_PATH = path.join(__dirname, '../../bin/showcase/data/land-50m.json');
const topology = JSON.parse(fs.readFileSync(DATA_PATH, 'utf8'));
const landGeojson = topojson.feature(topology, topology.objects.land);
const multi = landGeojson.features[0].geometry;

const polygon = multi.coordinates[93];
const geom = { type: 'Polygon', coordinates: polygon };

function runClip(name, clipFactory, radiusDeg) {
  const segments = [];
  let currentLine = null;
  const sink = {
    polygonStart() {},
    polygonEnd() {},
    lineStart() { currentLine = []; },
    lineEnd() { if (currentLine) segments.push(currentLine); currentLine = null; },
    point(x, y) { currentLine.push([x, y]); },
  };

  const projection = d3.geoOrthographic()
    .scale(100)
    .translate([0, 0])
    .center([0, 0])
    .rotate([45, -15, 0]);
  const rotate = d3.geoRotation([45, -15, 0]);

  // D3's clip receives points after rotation. We can apply rotation manually and
  // use a clip whose visible function/circle is centered at the origin.
  // For circle clip, use clipCircle(radius in radians).
  const clipStream = clipFactory(radiusDeg * Math.PI / 180)(sink);

  // Feed the rotated polygon through the clip stream.
  clipStream.polygonStart();
  for (const ring of polygon) {
    clipStream.lineStart();
    for (const p of ring) {
      const r = rotate(p);
      clipStream.point(r[0], r[1]);
    }
    clipStream.lineEnd();
  }
  clipStream.polygonEnd();

  console.log(`\n=== ${name} segments (${segments.length}) ===`);
  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    console.log(`seg ${i} len=${seg.length} first=[${seg[0].map(x => x*180/Math.PI).join(',')}] last=[${seg[seg.length-1].map(x => x*180/Math.PI).join(',')}]`);
  }
}

runClip('circle', clipCircle, 90 + 1e-6);
