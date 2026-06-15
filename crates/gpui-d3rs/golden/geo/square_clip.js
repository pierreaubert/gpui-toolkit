const clip = require('../node_modules/d3-geo/src/clip/index.js').default;
const clipCircle = require('../node_modules/d3-geo/src/clip/circle.js').default;

const polygon = {
  type: 'Polygon',
  coordinates: [[[-20,20],[20,20],[20,-20],[-20,-20],[-20,20]]]
};

const segments = [];
let currentLine = null;
const sink = {
  polygonStart() {},
  polygonEnd() {},
  lineStart() { currentLine = []; },
  lineEnd() { if (currentLine) segments.push(currentLine); currentLine = null; },
  point(x, y) { currentLine.push([x, y]); },
};

const clipStream = clipCircle(10 * Math.PI / 180)(sink);
clipStream.polygonStart();
for (const ring of polygon.coordinates) {
  clipStream.lineStart();
  for (const p of ring) clipStream.point(p[0] * Math.PI / 180, p[1] * Math.PI / 180);
  clipStream.lineEnd();
}
clipStream.polygonEnd();

console.log('segments', segments.length);
for (let i = 0; i < segments.length; i++) {
  console.log('seg', i, 'len', segments[i].length, 'first', segments[i][0].map(x=>x*180/Math.PI), 'last', segments[i][segments[i].length-1].map(x=>x*180/Math.PI));
}
