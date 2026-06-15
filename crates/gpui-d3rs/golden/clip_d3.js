const d3 = require('d3');
const fs = require('fs');
const topojson = require('topojson-client');
const data = JSON.parse(fs.readFileSync('/Volumes/home_ext1/src_pierre/gpui-toolkit/crates/gpui-d3rs/bin/showcase/data/land-50m.json', 'utf8'));
const land = topojson.feature(data, data.objects.land).features[0];
const polygon = land.geometry.coordinates[1200];
console.log('rings:', polygon.length, 'outer len', polygon[0].length);
const rotate = d3.geoRotation([60, -60, 0]);
function collect(orderLabel) {
  let pieces = [];
  let currentPiece = null;
  let currentRing = [];
  const outputSink = {
    point(x, y, z) {
      if (currentRing) currentRing.push([x, y]);
    },
    lineStart() {
      if (currentRing) currentRing.push(['__LS__']);
    },
    lineEnd() {
      if (currentRing) currentRing.push(['__LE__']);
    },
    polygonStart() {
      currentPiece = [];
    },
    polygonEnd() {
      if (currentPiece !== null) pieces.push(currentPiece);
      currentPiece = null;
    }
  };
  const clip = d3.geoClipAntimeridian(outputSink);
  // order A: rotate -> clip -> output
  const stream = rotate.stream(clip);
  stream.polygonStart();
  for (const ring of polygon) {
    stream.lineStart();
    for (const [lon, lat] of ring) {
      stream.point(lon, lat);
    }
    stream.lineEnd();
  }
  stream.polygonEnd();
  console.log('order', orderLabel, 'pieces:', pieces.length);
  for (let i = 0; i < pieces.length; i++) {
    const piece = pieces[i];
    let n = 0;
    for (const r of piece) n += r.length;
    console.log('piece', i, 'rings', piece.length, 'points', n, 'first', JSON.stringify(piece[0][0]), 'last', JSON.stringify(piece[0][piece[0].length-1]));
  }
  return pieces;
}
collect('A');
