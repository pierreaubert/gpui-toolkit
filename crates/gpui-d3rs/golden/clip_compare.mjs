import * as d3 from 'd3';

function identity(lambda, phi) { return [lambda, phi]; }
const p = d3.geoProjection(identity).scale(1).translate([0,0]).rotate([0,0,0]).center([0,0]);

const polygon = {
  type: 'Polygon',
  coordinates: [[[170,80],[-170,80],[-170,70],[170,70],[170,80]]]
};

const pts = [];
const sink = {
  polygonStart(){}, polygonEnd(){},
  lineStart(){}, lineEnd(){},
  point(x,y){ pts.push([x,y]); }
};
d3.geoStream(polygon, p.stream(sink));
console.log('D3 clipped points count', pts.length);
console.log('first 20:', JSON.stringify(pts.slice(0,20)));
console.log('near north pole', pts.filter(p => Math.abs(p[1]-Math.PI/2)<1e-3).length);
console.log('near south pole', pts.filter(p => Math.abs(p[1]+Math.PI/2)<1e-3).length);
console.log('y range', Math.min(...pts.map(p=>p[1])), Math.max(...pts.map(p=>p[1])));
