import clipAntimeridianFactory from './node_modules/d3-geo/src/clip/antimeridian.js';
import clipCircleFactory from './node_modules/d3-geo/src/clip/circle.js';
import { degrees } from './node_modules/d3-geo/src/math.js';
import fs from 'fs';

const RAD = Math.PI / 180;

function collectSink() {
  const lines = [];
  const rings = [];
  let current = [];
  let inPolygon = false;
  return {
    point(x, y) {
      current.push([x * degrees, y * degrees]);
    },
    lineStart() {
      current = [];
    },
    lineEnd() {
      if (current.length > 1) {
        if (inPolygon) rings.push(current);
        else lines.push(current);
      }
      current = [];
    },
    polygonStart() {
      inPolygon = true;
    },
    polygonEnd() {
      inPolygon = false;
    },
    result() {
      return { lines, rings };
    }
  };
}

function runClip(clipper, feature) {
  const sink = collectSink();
  const c = clipper(sink);
  if (feature.type === 'LineString') {
    c.lineStart();
    for (const [lon, lat] of feature.coordinates) {
      c.point(lon * RAD, lat * RAD);
    }
    c.lineEnd();
  } else if (feature.type === 'Polygon') {
    c.polygonStart();
    for (const ring of feature.coordinates) {
      c.lineStart();
      for (const [lon, lat] of ring) {
        c.point(lon * RAD, lat * RAD);
      }
      c.lineEnd();
    }
    c.polygonEnd();
  }
  return sink.result();
}

const antimeridian = clipAntimeridianFactory;

const lineCrossing = {
  type: 'LineString',
  coordinates: [[170, 65], [-170, 65]]
};
const antimeridianLine = runClip(antimeridian, lineCrossing);
fs.writeFileSync('expected_antimeridian_line.json', JSON.stringify(antimeridianLine, null, 2));

const polygonCrossing = {
  type: 'Polygon',
  coordinates: [[[170, 65], [175, 60], [-175, 60], [-170, 65], [170, 65]]]
};
const antimeridianPolygon = runClip(antimeridian, polygonCrossing);
fs.writeFileSync('expected_antimeridian_polygon.json', JSON.stringify(antimeridianPolygon, null, 2));

const squarePolygon = {
  type: 'Polygon',
  coordinates: [[[0, 0], [40, 0], [40, 40], [0, 40], [0, 0]]]
};

for (const deg of [30, 90]) {
  const circle = clipCircleFactory(deg * RAD);
  const result = runClip(circle, squarePolygon);
  fs.writeFileSync(`expected_circle_${deg}_polygon.json`, JSON.stringify(result, null, 2));
}

console.log('done');
