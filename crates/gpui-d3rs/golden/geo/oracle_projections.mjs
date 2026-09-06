import * as d3 from 'd3-geo';

const PTS = [[0,0],[30,20],[-120,45],[100,-30],[179,85],[-45,-60],[10,0],[-98,38],[0,60]];
const fmt = (v) => (v === null || v === undefined) ? null : +v.toFixed(9);
const proj = (p, pts) => pts.map(([lo, la]) => { const r = p([lo, la]); return r ? [fmt(r[0]), fmt(r[1])] : null; });

const out = {};

// A: raw-ish (scale 150, translate 0,0), defaults otherwise
for (const [name, mk] of [
  ['mercator', () => d3.geoMercator()],
  ['equirectangular', () => d3.geoEquirectangular()],
  ['orthographic', () => d3.geoOrthographic()],
  ['stereographic', () => d3.geoStereographic()],
  ['transverseMercator', () => d3.geoTransverseMercator()],
  ['conicEqualArea', () => d3.geoConicEqualArea()],
  ['albers', () => d3.geoAlbers()],
]) {
  const p = mk().scale(150).translate([0, 0]);
  out[name + '.plain'] = proj(p, PTS);
}

// B: center semantics
for (const [name, mk] of [
  ['mercator', () => d3.geoMercator()],
  ['equirectangular', () => d3.geoEquirectangular()],
  ['orthographic', () => d3.geoOrthographic()],
  ['conicEqualArea', () => d3.geoConicEqualArea()],
]) {
  const p = mk().scale(200).translate([100, 50]).center([10, 20]);
  out[name + '.center'] = proj(p, PTS);
}

// C: rotation
for (const [name, mk] of [
  ['mercator', () => d3.geoMercator()],
  ['orthographic', () => d3.geoOrthographic()],
]) {
  const p = mk().scale(200).translate([100, 50]).rotate([30, -20]);
  out[name + '.rotate'] = proj(p, PTS);
}

// D: center + rotate combined
{
  const p = d3.geoMercator().scale(200).translate([100, 50]).rotate([30, -20]).center([10, 20]);
  out['mercator.centerRotate'] = proj(p, PTS);
}

// E: albers defaults on US points
{
  const p = d3.geoAlbers(); // scale 1070, translate 480,250, rotate 96,0, center -0.6,38.7, parallels 29.5,45.5
  out['albers.defaults'] = proj(p, [[-98,38],[-120,45],[-74,40],[-100,30]]);
}

// F: fit methods on mercator for a test polygon
{
  const poly = {type:'Polygon', coordinates:[[[-10,-10],[10,-10],[10,10],[-10,10],[-10,-10]]]};
  const p1 = d3.geoMercator();
  p1.fitExtent([[10,20],[310,220]], poly);
  out['fit.extent'] = {scale: fmt(p1.scale()), translate: p1.translate().map(fmt)};
  const p2 = d3.geoMercator();
  p2.fitSize([300, 200], poly);
  out['fit.size'] = {scale: fmt(p2.scale()), translate: p2.translate().map(fmt)};
  const p3 = d3.geoMercator();
  p3.fitWidth(300, poly);
  out['fit.width'] = {scale: fmt(p3.scale()), translate: p3.translate().map(fmt)};
  const p4 = d3.geoMercator();
  p4.fitHeight(200, poly);
  out['fit.height'] = {scale: fmt(p4.scale()), translate: p4.translate().map(fmt)};
}

// G: transverseMercator raw sanity (spherical: swap of mercator)
{
  const p = d3.geoTransverseMercator().scale(150).translate([0,0]);
  out['transverseMercator.plain'] = proj(p, PTS);
}

// H: transverse with rotate/center
{
  const P = d3.geoTransverseMercator().scale(200).translate([100,50]).rotate([30,-20]);
  out['transverse.rotate'] = proj(P, PTS);
  const Q = d3.geoTransverseMercator().scale(200).translate([100,50]).rotate([30,-20,45]);
  out['transverse.rotateGamma'] = proj(Q, PTS);
  const C = d3.geoTransverseMercator().scale(200).translate([100,50]).center([10,20]);
  out['transverse.center'] = proj(C, PTS);
}

// I: invert spot checks (mercator + transverse with rotate)
{
  const P = d3.geoMercator().scale(200).translate([100,50]).rotate([30,-20]).center([10,20]);
  out['invert.mercator'] = [[300,100],[100,50]].map(([x,y]) => P.invert([x,y]).map(fmt));
  const Q = d3.geoTransverseMercator().scale(200).translate([100,50]).rotate([30,-20]).center([10,20]);
  out['invert.transverse'] = [[300,100],[100,50]].map(([x,y]) => Q.invert([x,y]).map(fmt));
}

// J: per-projection fitSize/fitExtent on the F-block test polygon
{
  const poly = {type:'Polygon', coordinates:[[[-10,-10],[10,-10],[10,10],[-10,10],[-10,-10]]]};
  const makers = {
    mercator: () => d3.geoMercator(),
    equirectangular: () => d3.geoEquirectangular(),
    orthographic: () => d3.geoOrthographic(),
    stereographic: () => d3.geoStereographic(),
    conicEqualArea: () => d3.geoConicEqualArea(),
    albers: () => d3.geoAlbers(),
    transverseMercator: () => d3.geoTransverseMercator(),
  };
  const cw = {type:'Polygon', coordinates:[[[-10,-10],[-10,10],[10,10],[10,-10],[-10,-10]]]};
  for (const [name, mk] of Object.entries(makers)) {
    const R = mk().fitSize([300,200], cw);
    out[`fitsize_tight.${name}`] = { scale: R.scale(), translate: R.translate().map(fmt) };
  }
  for (const [name, mk] of Object.entries(makers)) {
    const P = mk().fitSize([300,200], poly);
    out[`fitsize.${name}`] = { scale: P.scale(), translate: P.translate().map(fmt) };
    const Q = mk().fitExtent([[10,20],[310,220]], poly);
    out[`fitextent.${name}`] = { scale: Q.scale(), translate: Q.translate().map(fmt) };
  }
}
console.log(JSON.stringify(out));

