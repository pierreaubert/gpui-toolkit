/**
 * Golden file generator for D3.js example outputs
 * 
 * This generates JSON files containing outputs from D3.js examples
 * that can be compared against Rust implementations.
 * 
 * Usage:
 *   node generate_examples.js           # Generate all golden files
 *   node generate_examples.js force   # Generate only force tests
 *   node generate_examples.js hierarchy
 *   node generate_examples.js chord
 */

const d3 = require('d3');
const fs = require('fs');
const path = require('path');

const TOLERANCE = 1e-6;

function createGoldenFile(module, func, testCases) {
  return {
    module,
    function: func,
    d3_version: d3.version,
    tolerance: TOLERANCE,
    generated_at: new Date().toISOString(),
    test_cases: testCases
  };
}

// ============================================================================
// FORCE SIMULATION
// ============================================================================

function generateForceTests() {
  const testCases = [];

  // Basic force simulation with predefined seed for reproducibility
  {
    const nodes = [
      { id: 'a', group: 1 },
      { id: 'b', group: 1 },
      { id: 'c', group: 2 },
      { id: 'd', group: 2 },
      { id: 'e', group: 3 }
    ];
    
    const links = [
      { source: 'a', target: 'b' },
      { source: 'b', target: 'c' },
      { source: 'c', target: 'd' },
      { source: 'd', target: 'e' }
    ];
    
    // Create simulation
    const simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(links).id(d => d.id).distance(30))
      .force('charge', d3.forceManyBody().strength(-100))
      .force('center', d3.forceCenter(50, 50))
      .stop();
    
    // Run for fixed number of ticks
    for (let i = 0; i < 100; i++) {
      simulation.tick();
    }
    
    testCases.push({
      name: "basic_force_simulation",
      nodes: nodes.map(n => ({
        id: n.id,
        group: n.group,
        x: Math.round(n.x * 1000) / 1000,
        y: Math.round(n.y * 1000) / 1000,
        vx: Math.round(n.vx * 1000) / 1000,
        vy: Math.round(n.vy * 1000) / 1000
      })),
      links: links.map(l => ({
        source: typeof l.source === 'object' ? l.source.id : l.source,
        target: typeof l.target === 'object' ? l.target.id : l.target
      })),
      iterations: 100
    });
  }
  
  // Force many body
  {
    const nodes = Array.from({ length: 20 }, (_, i) => ({ id: i, group: i % 3 }));
    
    const simulation = d3.forceSimulation(nodes)
      .force('charge', d3.forceManyBody().strength(-30))
      .force('x', d3.forceX(50).strength(0.05))
      .force('y', d3.forceY(50).strength(0.05))
      .stop();
    
    for (let i = 0; i < 50; i++) {
      simulation.tick();
    }
    
    testCases.push({
      name: "force_many_body",
      node_count: nodes.length,
      nodes: nodes.slice(0, 5).map(n => ({
        id: n.id,
        x: Math.round(n.x * 1000) / 1000,
        y: Math.round(n.y * 1000) / 1000
      })),
      iterations: 50
    });
  }
  
  // Force collision
  {
    const nodes = Array.from({ length: 10 }, (_, i) => ({ id: i, radius: 5 + Math.random() * 10 }));
    
    const simulation = d3.forceSimulation(nodes)
      .force('collision', d3.forceCollide().radius(d => d.radius).strength(1))
      .force('x', d3.forceX(50).strength(0.1))
      .force('y', d3.forceY(50).strength(0.1))
      .stop();
    
    for (let i = 0; i < 30; i++) {
      simulation.tick();
    }
    
    testCases.push({
      name: "force_collision",
      nodes: nodes.map(n => ({
        id: n.id,
        radius: n.radius,
        x: Math.round(n.x * 1000) / 1000,
        y: Math.round(n.y * 1000) / 1000
      })),
      iterations: 30
    });
  }
  
  // Radial force
  {
    const nodes = Array.from({ length: 15 }, (_, i) => ({ id: i, group: i % 3 }));
    
    const simulation = d3.forceSimulation(nodes)
      .force('radial', d3.forceRadial(30, 50, 50).strength(0.5))
      .force('charge', d3.forceManyBody().strength(-20))
      .stop();
    
    for (let i = 0; i < 30; i++) {
      simulation.tick();
    }
    
    testCases.push({
      name: "force_radial",
      center: [50, 50],
      radius: 30,
      nodes: nodes.slice(0, 5).map(n => ({
        id: n.id,
        x: Math.round(n.x * 1000) / 1000,
        y: Math.round(n.y * 1000) / 1000
      })),
      iterations: 30
    });
  }

  const golden = createGoldenFile("d3-force", "forceSimulation", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'force.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/force.json');
}

// ============================================================================
// HIERARCHY LAYOUTS
// ============================================================================

function generateHierarchyTests() {
  const testCases = [];
  
  // Create sample hierarchy data
  const flareData = {
    name: "flare",
    children: [
      {
        name: "analytics",
        children: [
          { name: "cluster", children: [{ name: "AgglomerativeCluster" }, { name: "CommunityStructure" }] },
          { name: "graph", children: [{ name: "BetweennessCentrality" }, { name: "LinkDistance" }] },
          { name: "optimization", children: [{ name: "AspectRatioLayout" }] }
        ]
      },
      {
        name: "animate",
        children: [
          { name: "interpolate", children: [{ name: "ArrayInterpolator" }, { name: "ColorInterpolator" }] },
          { name: "transition", children: [{ name: "Transition" }, { name: "ShapeTransition" }] }
        ]
      },
      { name: "data", children: [{ name: "DataSchema" }, { name: "DataList" }] }
    ]
  };

  // Create hierarchy root
  const root = d3.hierarchy(flareData);
  
  // Count nodes
  const count = d3.hierarchy(flareData).count();
  testCases.push({
    name: "hierarchy_count",
    total_nodes: count.value,
    leaf_count: count.leaves().length
  });
  
  // Sum values
  const sumRoot = d3.hierarchy(flareData)
    .sum(d => d.children ? 0 : 1)
    .sort((a, b) => b.value - a.value);
  
  testCases.push({
    name: "hierarchy_sum",
    root_value: sumRoot.value,
    max_depth: sumRoot.height,
    leaves: sumRoot.leaves().map(l => ({
      data: l.data.name,
      value: l.value,
      depth: l.depth
    }))
  });

  // Treemap layout
  const treemap = d3.treemap()
    .size([100, 100])
    .padding(1);
  
  const treemapRoot = treemap(sumRoot);
  
  testCases.push({
    name: "treemap_layout",
    size: [100, 100],
    padding: 1,
    leaves: treemapRoot.leaves().slice(0, 5).map(l => ({
      data: l.data.name,
      x0: Math.round(l.x0 * 1000) / 1000,
      y0: Math.round(l.y0 * 1000) / 1000,
      x1: Math.round(l.x1 * 1000) / 1000,
      y1: Math.round(l.y1 * 1000) / 1000,
      value: l.value
    }))
  });

  // Treemap with different padding
  const treemap2 = d3.treemap()
    .size([100, 100])
    .paddingInner(2)
    .paddingOuter(4);
  
  const treemapRoot2 = treemap2(sumRoot);
  
  testCases.push({
    name: "treemap_with_padding",
    padding_inner: 2,
    padding_outer: 4,
    leaves: treemapRoot2.leaves().slice(0, 3).map(l => ({
      data: l.data.name,
      x0: Math.round(l.x0 * 1000) / 1000,
      y0: Math.round(l.y0 * 1000) / 1000,
      x1: Math.round(l.x1 * 1000) / 1000,
      y1: Math.round(l.y1 * 1000) / 1000
    }))
  });

  // Pack layout (circle packing)
  const pack = d3.pack()
    .size([100, 100])
    .padding(2);
  
  const packRoot = pack(d3.hierarchy(flareData).sum(d => d.children ? 0 : 1));
  
  testCases.push({
    name: "pack_layout",
    size: [100, 100],
    padding: 2,
    leaves: packRoot.leaves().slice(0, 5).map(l => ({
      data: l.data.name,
      x: Math.round(l.x * 1000) / 1000,
      y: Math.round(l.y * 1000) / 1000,
      r: Math.round(l.r * 1000) / 1000
    }))
  });

  // Partition layout (sunburst)
  const partition = d3.partition()
    .size([100, 100]);
  
  const partitionRoot = partition(d3.hierarchy(flareData).sum(d => d.children ? 0 : 1));
  
  testCases.push({
    name: "partition_layout",
    leaves: partitionRoot.leaves().slice(0, 5).map(l => ({
      data: l.data.name,
      x0: Math.round(l.x0 * 1000) / 1000,
      y0: Math.round(l.y0 * 1000) / 1000,
      x1: Math.round(l.x1 * 1000) / 1000,
      y1: Math.round(l.y1 * 1000) / 1000
    }))
  });

  // Tree layout
  const tree = d3.tree()
    .size([100, 80]);
  
  const treeRoot = tree(d3.hierarchy(flareData));
  
  testCases.push({
    name: "tree_layout",
    size: [100, 80],
    nodes: treeRoot.descendants().slice(0, 6).map(n => ({
      data: n.data.name,
      x: Math.round(n.x * 1000) / 1000,
      y: Math.round(n.y * 1000) / 1000,
      depth: n.depth
    }))
  });

  // Cluster layout
  const cluster = d3.cluster()
    .size([100, 80]);
  
  const clusterRoot = cluster(d3.hierarchy(flareData));
  
  testCases.push({
    name: "cluster_layout",
    leaves: clusterRoot.leaves().map(l => ({
      data: l.data.name,
      x: Math.round(l.x * 1000) / 1000,
      y: Math.round(l.y * 1000) / 1000
    }))
  });

  const golden = createGoldenFile("d3-hierarchy", "hierarchy", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'hierarchy.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/hierarchy.json');
}

// ============================================================================
// CHORD DIAGRAM
// ============================================================================

function generateChordTests() {
  const testCases = [];

  // Sample matrix data
  const matrix = [
    [0,  5,  6,  4,  7,  4],
    [5,  0,  5,  4,  6,  5],
    [6,  5,  0,  8,  2,  7],
    [4,  4,  8,  0,  5,  6],
    [7,  6,  2,  5,  0,  9],
    [4,  5,  7,  6,  9,  0]
  ];
  
  const names = ["A", "B", "C", "D", "E", "F"];
  
  // Create chord layout
  const chord = d3.chord()
    .padAngle(0.05)
    .sortSubgroups(d3.descending);
  
  const chords = chord(matrix);
  
  testCases.push({
    name: "chord_basic",
    matrix_size: matrix.length,
    pad_angle: 0.05,
    groups: chords.groups.map((g, i) => ({
      index: g.startIndex,
      name: names[i],
      value: g.value,
      start_angle: Math.round(g.startAngle * 1000) / 1000,
      end_angle: Math.round(g.endAngle * 1000) / 1000
    })),
    pairs: chords.slice(0, 6).map(c => ({
      source: {
        index: c.source.index,
        start_angle: Math.round(c.source.startAngle * 1000) / 1000,
        end_angle: Math.round(c.source.endAngle * 1000) / 1000
      },
      target: {
        index: c.target.index,
        start_angle: Math.round(c.target.startAngle * 1000) / 1000,
        end_angle: Math.round(c.target.endAngle * 1000) / 1000
      }
    }))
  });

  // Chord with different pad angle
  const chord2 = d3.chord()
    .padAngle(0.1)
    .sortSubgroups(null);
  
  const chords2 = chord2(matrix);
  
  testCases.push({
    name: "chord_no_sort",
    pad_angle: 0.1,
    groups: chords2.groups.map((g, i) => ({
      index: g.startIndex,
      value: g.value,
      start_angle: Math.round(g.startAngle * 1000) / 1000,
      end_angle: Math.round(g.endAngle * 1000) / 1000
    }))
  });

  const golden = createGoldenFile("d3-chord", "chord", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'chord.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/chord.json');
}

// ============================================================================
// GEO
// ============================================================================

function generateGeoTests() {
  const testCases = [];

  // Simple GeoJSON point
  const point = [-122.4194, 37.7749]; // San Francisco
  
  testCases.push({
    name: "geo_point",
    coordinates: point,
    mercator: d3.geoMercator()(point),
    equirectangular: d3.geoEquirectangular()(point),
    orthographic: d3.geoOrthographic()(point)
  });

  // GeoJSON LineString (great arc)
  const lineString = {
    type: "LineString",
    coordinates: [[-122.4194, 37.7749], [0.1278, 51.5074]] // SF to London
  };
  
  testCases.push({
    name: "geo_line",
    type: "LineString",
    coordinates: lineString.coordinates,
    length: d3.geoLength(lineString),
    bounds: d3.geoBounds(lineString),
    centroid: d3.geoCentroid(lineString)
  });

  // GeoJSON Polygon
  const polygon = {
    type: "Polygon",
    coordinates: [[
      [-122.4194, 37.7749],
      [-122.4194, 37.8049],
      [-122.3894, 37.8049],
      [-122.3894, 37.7749],
      [-122.4194, 37.7749]
    ]]
  };
  
  testCases.push({
    name: "geo_polygon",
    type: "Polygon",
    area: d3.geoArea(polygon),
    bounds: d3.geoBounds(polygon),
    centroid: d3.geoCentroid(polygon)
  });

  // Projection transforms
  const projection = d3.geoMercator()
    .scale(100)
    .translate([50, 50]);
  
  testCases.push({
    name: "geo_projection",
    projection: "mercator",
    scale: 100,
    translate: [50, 50],
    forward: projection([-122.4194, 37.7749]),
    invert: projection.invert([50, 50])
  });

  // Stereographic projection
  const stereo = d3.geoStereographic()
    .scale(100)
    .translate([50, 50]);
  
  testCases.push({
    name: "geo_stereographic",
    projection: "stereographic",
    forward: stereo([-122.4194, 37.7749]),
    invert: stereo.invert([50, 50])
  });

  // Conic projection
  const albers = d3.geoAlbers()
    .scale(100)
    .translate([50, 50])
    .parallels([29.5, 45.5])
    .rotate([96, 0])
    .center([0, 38])
    .translate([50, 50]);
  
  testCases.push({
    name: "geo_albers",
    projection: "albers",
    parallels: [29.5, 45.5],
    rotation: [-96, 0],
    center: [0, 38],
    forward: albers([-96, 38])
  });

  const golden = createGoldenFile("d3-geo", "geo", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'geo.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/geo.json');
}

// ============================================================================
// COLOR SCALES (d3-scale-chromatic)
// ============================================================================

function generateColorScaleTests() {
  const testCases = [];

  // Sequential single-hue
  const schemes = ['Blues', 'Greens', 'Reds', 'Purples', 'Oranges'];
  for (const schemeName of schemes) {
    const scheme = d3[`scheme${schemeName}`][9];
    testCases.push({
      name: `sequential_${schemeName.toLowerCase()}`,
      scheme: schemeName,
      colors: scheme
    });
  }

  // Sequential multi-hue
  const multiHue = ['Viridis', 'Magma', 'Inferno', 'Plasma', 'Turbo'];
  for (const name of multiHue) {
    const scale = d3[`interpolate${name}`];
    testCases.push({
      name: `sequential_${name.toLowerCase()}`,
      samples: [0, 0.25, 0.5, 0.75, 1].map(t => scale(t))
    });
  }

  // Diverging schemes
  const diverging = ['RdBu', 'RdYlBu', 'RdYlGn', 'PiYG', 'BrBG', 'PuOr'];
  for (const name of diverging) {
    const scheme = d3[`scheme${name}`][11];
    testCases.push({
      name: `diverging_${name}`,
      colors: scheme
    });
  }

  // Cyclical schemes - use interpolateRainbow as example
  testCases.push({
    name: 'cyclical_rainbow',
    samples: [0, 0.25, 0.5, 0.75, 1].map(t => d3.interpolateRainbow(t))
  });

  const golden = createGoldenFile("d3-scale-chromatic", "colorScales", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'color_scales.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/color_scales.json');
}

// ============================================================================
// DRAG INTERACTION
// ============================================================================

function generateDragTests() {
  const testCases = [];

  // Drag behavior creation
  const drag = d3.drag()
    .container(function() { return this; })
    .touchable(function() { return true; })
    .subject(function(event) { return event; });
  
  testCases.push({
    name: "drag_creation",
    container: "function",
    touchable: "function", 
    subject: "function"
  });

  // Drag event simulation
  const subject = { x: 10, y: 20, fx: null, fy: null };
  
  // Simulate drag start
  subject.fx = subject.x;
  subject.fy = subject.y;
  
  testCases.push({
    name: "drag_start",
    initial: { x: 10, y: 20 },
    after_start: { fx: subject.fx, fy: subject.fy }
  });

  // Simulate drag
  subject.fx = 25;
  subject.fy = 35;
  
  testCases.push({
    name: "drag_drag",
    position: { x: 25, y: 35 },
    result: { fx: subject.fx, fy: subject.fy }
  });

  // Simulate drag end
  subject.fx = null;
  subject.fy = null;
  
  testCases.push({
    name: "drag_end",
    after_end: { fx: subject.fx, fy: subject.fy }
  });

  // Drag with filter
  const dragFiltered = d3.drag()
    .filter(function(event) { return !event.ctrlKey; });
  
  testCases.push({
    name: "drag_filter",
    filter: "function"
  });

  // Drag click distance
  const dragClickDist = d3.drag()
    .clickDistance(5);
  
  testCases.push({
    name: "drag_click_distance",
    distance: 5
  });

  const golden = createGoldenFile("d3-drag", "drag", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'drag.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/drag.json');
}

// ============================================================================
// BRUSH INTERACTION
// ============================================================================

function generateBrushTests() {
  const testCases = [];

  // Basic brush creation
  const brush = d3.brush()
    .extent([[0, 0], [100, 100]])
    .on("start brush end", () => {});
  
  // Test brush extent
  testCases.push({
    name: "brush_basic",
    extent: [[0, 0], [100, 100]],
    // Just verify brush can be created with extent
    created: true
  });

  // Brush with different mode
  const brush2 = d3.brushX()
    .extent([[0, 0], [100, 20]]);
  
  testCases.push({
    name: "brush_horizontal",
    extent: [[0, 0], [100, 20]],
    created: true
  });

  // Brush selection boundaries
  const brush3 = d3.brush()
    .extent([[0, 0], [100, 100]])
    .filter(() => false); // Disable default filter
  
  testCases.push({
    name: "brush_boundaries",
    extent: [[0, 0], [100, 100]],
    handle_size: 6, // default
    created: true
  });

  const golden = createGoldenFile("d3-brush", "brush", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'brush.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/brush.json');
}

// ============================================================================
// ZOOM INTERACTION
// ============================================================================

function generateZoomTests() {
  const testCases = [];

  // Basic zoom identity
  const zoomIdentity = d3.zoomIdentity;
  
  testCases.push({
    name: "zoom_identity",
    x: zoomIdentity.x,
    y: zoomIdentity.y,
    k: zoomIdentity.k,
    toString: zoomIdentity.toString()
  });

  // Zoom transform with translate
  const transform = d3.zoomIdentity.translate(10, 20).scale(1.5);
  
  testCases.push({
    name: "zoom_transform",
    x: transform.x,
    y: transform.y,
    k: transform.k,
    apply: transform.apply([0, 0]),
    applyX: transform.applyX(10),
    applyY: transform.applyY(20),
    invert: transform.invert([25, 50]),
    invertX: transform.invertX(25),
    invertY: transform.invertY(50)
  });

  // Zoom scale extent
  const zoom = d3.zoom()
    .scaleExtent([0.1, 10])
    .translateExtent([[0, 0], [100, 100]]);
  
  testCases.push({
    name: "zoom_configuration",
    scaleExtent: [0.1, 10],
    translateExtent: [[0, 0], [100, 100]],
    filter: zoom.filter() ? "default" : "none"
  });

  // Test zoom transform string
  const transformStr = transform.toString();
  
  testCases.push({
    name: "zoom_to_string",
    transform: transformStr
  });

  const golden = createGoldenFile("d3-zoom", "zoom", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'zoom.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/zoom.json');
}

// ============================================================================
// SANKEY DIAGRAM (d3-sankey)
// ============================================================================

function generateSankeyTests() {
  const testCases = [];
  const { sankey, sankeyLinkHorizontal } = require('d3-sankey');

  // Basic sankey with energy data
  {
    const nodes = [
      { name: "Agricultural 'Waste'" },
      { name: "Bio-conversion" },
      { name: "Livestock" },
      { name: "Land" },
      { name: "Solar" },
      { name: "Imports" }
    ];
    
    const links = [
      { source: 0, target: 1, value: 124.729 },
      { source: 2, target: 1, value: 35.0 },
      { source: 1, target: 3, value: 6.0 },
      { source: 1, target: 4, value: 20.0 },
      { source: 1, target: 5, value: 81.144 }
    ];
    
    const graph = sankey()
      .nodeWidth(15)
      .nodePadding(10)
      .extent([[1, 1], [100, 100]])({
        nodes: nodes.map(d => Object.assign({}, d)),
        links: links.map(d => Object.assign({}, d))
      });
    
    testCases.push({
      name: "basic_sankey",
      node_width: 15,
      node_padding: 10,
      extent: [[1, 1], [100, 100]],
      nodes: graph.nodes.map(n => ({
        name: n.name,
        x0: Math.round(n.x0 * 1000) / 1000,
        x1: Math.round(n.x1 * 1000) / 1000,
        y0: Math.round(n.y0 * 1000) / 1000,
        y1: Math.round(n.y1 * 1000) / 1000,
        value: n.value
      })),
      links: graph.links.slice(0, 3).map(l => ({
        source: l.source.name,
        target: l.target.name,
        value: l.value,
        y0: Math.round(l.y0 * 1000) / 1000,
        y1: Math.round(l.y1 * 1000) / 1000
      }))
    });
  }

  // Simple sankey with numeric node IDs
  {
    const nodes = [
      { name: "A" }, { name: "B" }, { name: "C" }, { name: "D" }
    ];
    
    const links = [
      { source: 0, target: 2, value: 10 },
      { source: 1, target: 2, value: 5 },
      { source: 2, target: 3, value: 15 }
    ];
    
    const graph = sankey()
      .nodeId(d => d.index)
      .extent([[0, 0], [80, 60]])({
        nodes: nodes.map(d => Object.assign({}, d)),
        links: links.map(d => Object.assign({}, d))
      });
    
    testCases.push({
      name: "sankey_simple",
      node_count: nodes.length,
      link_count: links.length,
      nodes: graph.nodes.map(n => ({
        index: n.index,
        name: n.name,
        x0: Math.round(n.x0 * 1000) / 1000,
        y0: Math.round(n.y0 * 1000) / 1000,
        y1: Math.round(n.y1 * 1000) / 1000
      })),
      links: graph.links.map(l => ({
        source: l.source.index,
        target: l.target.index,
        value: l.value,
        width: Math.round(l.width * 1000) / 1000
      }))
    });
  }

  const golden = createGoldenFile("d3-sankey", "sankey", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'sankey.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/sankey.json');
}

// ============================================================================
// CALENDAR HEATMAP (d3-calendar)
// ============================================================================

function generateCalendarTests() {
  const testCases = [];

  // Generate sample time series data for a year
  const generateData = (year) => {
    const data = [];
    const start = new Date(year, 0, 1);
    const end = new Date(year, 11, 31);
    for (let d = new Date(start); d <= end; d.setDate(d.getDate() + 1)) {
      data.push({
        date: new Date(d),
        value: Math.floor(Math.random() * 100)
      });
    }
    return data;
  };

  // Calendar cell size
  const cellSize = 10;
  
  testCases.push({
    name: "calendar_cell_size",
    cell_size: cellSize,
    year: 2024
  });

  // Calculate calendar path data
  const year = 2024;
  const data = generateData(year);
  const startDate = new Date(year, 0, 1);
  const endDate = new Date(year, 11, 31);
  
  // Group data by week
  const weeks = [];
  let currentWeek = [];
  let currentDate = new Date(startDate);
  
  // Find first Sunday
  while (currentDate.getDay() !== 0) {
    currentDate.setDate(currentDate.getDate() + 1);
  }
  
  const firstSunday = new Date(currentDate);
  
  testCases.push({
    name: "calendar_2024",
    year: year,
    start_date: startDate.toISOString(),
    end_date: endDate.toISOString(),
    first_sunday: firstSunday.toISOString(),
    total_days: data.length
  });

  const golden = createGoldenFile("d3", "calendar", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'calendar.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/calendar.json');
}

// ============================================================================
// RADIAL LINE (d3.lineRadial from d3-shape)
// ============================================================================

function generateRadialLineTests() {
  const testCases = [];

  // Basic radial line
  const data = [
    [0, 10],
    [Math.PI * 0.25, 35],
    [Math.PI * 0.5, 55],
    [Math.PI * 0.75, 60],
    [Math.PI, 65],
    [Math.PI * 1.25, 70],
    [Math.PI * 1.5, 75],
    [Math.PI * 1.75, 80],
    [Math.PI * 2, 85]
  ];

  const lineRadial = d3.lineRadial()
    .angle(d => d[0])
    .radius(d => d[1])
    .curve(d3.curveLinear);

  testCases.push({
    name: "radial_line_basic",
    angle_count: data.length,
    path: lineRadial(data),
    sample_angles: data.slice(0, 3).map(d => d[0]),
    sample_radii: data.slice(0, 3).map(d => d[1])
  });

  // Radial line with curveBasis
  const lineRadialCurve = d3.lineRadial()
    .angle(d => d[0])
    .radius(d => d[1])
    .curve(d3.curveBasis);

  testCases.push({
    name: "radial_line_curve_basis",
    path: lineRadialCurve(data)
  });

  // Radial line with curveCardinalClosed
  const lineRadialCardinal = d3.lineRadial()
    .angle(d => d[0])
    .radius(d => d[1])
    .curve(d3.curveCardinalClosed);

  testCases.push({
    name: "radial_line_curve_cardinal_closed",
    path: lineRadialCardinal(data)
  });

  // Radial area (filled region)
  const areaRadial = d3.areaRadial()
    .angle(d => d[0])
    .innerRadius(10)
    .outerRadius(d => d[1])
    .curve(d3.curveLinear);

  testCases.push({
    name: "radial_area_basic",
    inner_radius: 10,
    path: areaRadial(data)
  });

  const golden = createGoldenFile("d3-shape", "lineRadial", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'radial_line.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/radial_line.json');
}

// ============================================================================
// HEXBIN (d3-hexbin)
// ============================================================================

function generateHexbinTests() {
  const testCases = [];
  const { hexbin } = require('d3-hexbin');

  // Simple hexbin with random data
  {
    const width = 100;
    const height = 100;
    const radius = 10;
    
    // Generate some deterministic data for reproducibility
    const data = [];
    for (let i = 0; i < 50; i++) {
      data.push([
        (Math.sin(i) * 0.5 + 0.5) * width,
        (Math.cos(i) * 0.5 + 0.5) * height
      ]);
    }

    const hex = hexbin()
      .extent([[0, 0], [width, height]])
      .radius(radius);
    
    const bins = hex(data);
    
    testCases.push({
      name: "basic_hexbin",
      width,
      height,
      radius,
      data_count: data.length,
      data: data,
      bins: bins.map(b => ({
        x: b.x,
        y: b.y,
        count: b.length
      }))
    });

    // Test with accessor functions
    const dataWithProps = data.map(d => ({ longitude: d[0], latitude: d[1] }));
    const hex2 = hexbin()
      .x(d => d.longitude)
      .y(d => d.latitude)
      .radius(5);
    
    const bins2 = hex2(dataWithProps);
    
    testCases.push({
      name: "hexbin_accessors",
      data: dataWithProps,
      radius: 5,
      bin_count: bins2.length,
      bins: bins2.slice(0, 5).map(b => ({
        x: b.x,
        y: b.y,
        count: b.length
      }))
    });
  }

  const golden = createGoldenFile("d3-hexbin", "hexbin", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'hexbin.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/hexbin.json');
}

// ============================================================================
// PARALLEL COORDINATES
// ============================================================================

function generateParallelCoordinatesTests() {
  const testCases = [];

  // Sample car data similar to common examples
  const data = [
    { name: "Toyota", mpg: 30, cylinders: 4, displacement: 91, weight: 2000, acceleration: 14 },
    { name: "Honda", mpg: 35, cylinders: 4, displacement: 79, weight: 1900, acceleration: 15 },
    { name: "Ford", mpg: 20, cylinders: 6, displacement: 170, weight: 2800, acceleration: 12 },
    { name: "Chevy", mpg: 18, cylinders: 8, displacement: 307, weight: 3500, acceleration: 11 },
    { name: "BMW", mpg: 25, cylinders: 4, displacement: 121, weight: 2500, acceleration: 13 },
    { name: "Audi", mpg: 28, cylinders: 4, displacement: 97, weight: 2100, acceleration: 14 }
  ];

  // Get dimensions (excluding name)
  const dimensions = Object.keys(data[0]).filter(k => k !== "name");

  testCases.push({
    name: "parallel_coords_dimensions",
    dimensions: dimensions,
    row_count: data.length
  });

  // Create scales for each dimension
  const yScales = {};
  for (const dim of dimensions) {
    const values = data.map(d => d[dim]);
    const extent = d3.extent(values);
    yScales[dim] = {
      extent: extent,
      domain: extent
    };
  }

  testCases.push({
    name: "parallel_coords_scales",
    scales: Object.fromEntries(
      dimensions.map(dim => [dim, yScales[dim]])
    )
  });

  // Generate path data for each row
  const xScale = d3.scalePoint()
    .domain(dimensions)
    .range([0, 100]);

  const pathData = data.map(row => {
    const points = dimensions.map(dim => [xScale(dim), yScales[dim].domain[1] - row[dim] + yScales[dim].domain[0]]);
    return {
      name: row.name,
      points: points
    };
  });

  testCases.push({
    name: "parallel_coords_paths",
    paths: pathData.slice(0, 3).map(p => ({
      name: p.name,
      point_count: p.points.length,
      first_point: p.points[0],
      last_point: p.points[p.points.length - 1]
    }))
  });

  const golden = createGoldenFile("d3", "parallelCoordinates", testCases);
  fs.mkdirSync(path.join(__dirname, 'examples'), { recursive: true });
  fs.writeFileSync(path.join(__dirname, 'examples', 'parallel_coordinates.json'), JSON.stringify(golden, null, 2));
  console.log('Generated: examples/parallel_coordinates.json');
}

// ============================================================================
// MAIN
// ============================================================================

const args = process.argv.slice(2);

if (args.length === 0) {
  console.log('Generating all D3 example golden files...');
  generateForceTests();
  generateHierarchyTests();
  generateChordTests();
  generateGeoTests();
  generateColorScaleTests();
  generateDragTests();
  generateBrushTests();
  generateZoomTests();
  generateSankeyTests();
  generateCalendarTests();
  generateRadialLineTests();
  generateParallelCoordinatesTests();
  generateHexbinTests();
  console.log('Done!');
} else {
  for (const arg of args) {
    switch (arg) {
      case 'force':
        generateForceTests();
        break;
      case 'hierarchy':
        generateHierarchyTests();
        break;
      case 'chord':
        generateChordTests();
        break;
      case 'geo':
        generateGeoTests();
        break;
      case 'colors':
      case 'color':
        generateColorScaleTests();
        break;
      case 'drag':
        generateDragTests();
        break;
      case 'brush':
        generateBrushTests();
        break;
      case 'zoom':
        generateZoomTests();
        break;
      case 'sankey':
        generateSankeyTests();
        break;
      case 'calendar':
        generateCalendarTests();
        break;
      case 'radial':
      case 'radial_line':
        generateRadialLineTests();
        break;
      case 'parallel':
      case 'parallel_coordinates':
        generateParallelCoordinatesTests();
        break;
      case 'hexbin':
        generateHexbinTests();
        break;
      default:
        console.log(`Unknown module: ${arg}`);
    }
  }
}
