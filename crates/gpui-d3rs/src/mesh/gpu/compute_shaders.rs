//! Adapter-side compute shader sources.
//!
//! The CPU implementation in [`super::compute`] remains the golden reference
//! for headless and unsupported-adapter builds. These kernels use the same
//! contracts: reductions ignore invalid scalar values and contour crossings
//! are edge-indexed so adjacent triangles share exactly one interpolated edge
//! point.

/// WGSL reduction and edge-intersection kernels.
pub const MESH_COMPUTE_WGSL: &str = r#"
struct PartialRange {
    min_value: f32,
    max_value: f32,
    valid: u32,
    _padding: u32,
};

@group(0) @binding(0) var<storage, read> values: array<f32>;
@group(0) @binding(1) var<storage, read_write> partial_ranges: array<PartialRange>;

var<workgroup> local_min: array<f32, 256>;
var<workgroup> local_max: array<f32, 256>;
var<workgroup> local_valid: array<u32, 256>;

@compute @workgroup_size(256)
fn field_min_max(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let lane = local_id.x;
    let count = arrayLength(&values);
    var minimum = 3.402823e+38;
    var maximum = -3.402823e+38;
    var valid = 0u;
    var index = global_id.x;
    loop {
        if (index >= count) { break; }
        let value = values[index];
        if (!isNan(value) && !isInf(value)) {
            minimum = min(minimum, value);
            maximum = max(maximum, value);
            valid = 1u;
        }
        index += 256u;
    }
    local_min[lane] = minimum;
    local_max[lane] = maximum;
    local_valid[lane] = valid;
    workgroupBarrier();
    if (lane == 0u) {
        var result_min = local_min[0];
        var result_max = local_max[0];
        var result_valid = local_valid[0];
        for (var lane_index = 1u; lane_index < 256u; lane_index += 1u) {
            result_min = min(result_min, local_min[lane_index]);
            result_max = max(result_max, local_max[lane_index]);
            result_valid = max(result_valid, local_valid[lane_index]);
        }
        partial_ranges[workgroup_id.x] = PartialRange(result_min, result_max, result_valid, 0u);
    }
}

struct Edge {
    a: u32,
    b: u32,
};
struct EdgeHit {
    // vec4 keeps the storage-array stride explicit for the Rust readback
    // contract. The w component is padding.
    position: vec4<f32>,
    valid: u32,
    _padding: u32,
};

@group(0) @binding(2) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> edges: array<Edge>;
@group(0) @binding(4) var<storage, read_write> edge_hits: array<EdgeHit>;
@group(0) @binding(5) var<uniform> level: f32;

@compute @workgroup_size(256)
fn edge_intersections(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&edges)) { return; }
    let edge = edges[id.x];
    let a = values[edge.a];
    let b = values[edge.b];
    if (isNan(a) || isNan(b) || isInf(a) || isInf(b)) {
        edge_hits[id.x].valid = 0u;
        return;
    }
    // This must stay identical to MarchingTriangles::edge_hit: an on-level
    // vertex is classified as high, including the zero-length-segment case.
    if ((a >= level) == (b >= level)) {
        edge_hits[id.x].valid = 0u;
        return;
    }
    let interpolation = clamp((level - a) / (b - a), 0.0, 1.0);
    edge_hits[id.x] = EdgeHit(
        vec4<f32>(mix(positions[edge.a].xyz, positions[edge.b].xyz, interpolation), 0.0),
        1u,
        0u,
    );
}

struct TriangleEdges {
    e0: u32,
    e1: u32,
    e2: u32,
};
struct Segment {
    start: vec4<f32>,
    end: vec4<f32>,
    valid: u32,
    _padding: u32,
};

@group(0) @binding(6) var<storage, read> triangle_edges: array<TriangleEdges>;
@group(0) @binding(7) var<storage, read_write> segments: array<Segment>;

// Gather is intentionally a separate pass. Each unique edge has exactly one
// interpolation result, so adjacent triangles cannot develop cracks. The
// edge pass above uses `>=` for the high-side classification; an on-level
// vertex with two below-level neighbours therefore produces the same
// zero-length segment as the CPU golden reference.
@compute @workgroup_size(256)
fn triangle_segments(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&triangle_edges)) { return; }
    let triangle = triangle_edges[id.x];
    let hit0 = edge_hits[triangle.e0];
    let hit1 = edge_hits[triangle.e1];
    let hit2 = edge_hits[triangle.e2];
    var first = vec4<f32>(0.0);
    var second = vec4<f32>(0.0);
    var count = 0u;
    if (hit0.valid != 0u) {
        first = hit0.position;
        count += 1u;
    }
    if (hit1.valid != 0u) {
        if (count == 0u) { first = hit1.position; }
        else if (count == 1u) { second = hit1.position; }
        count += 1u;
    }
    if (hit2.valid != 0u) {
        if (count == 0u) { first = hit2.position; }
        else if (count == 1u) { second = hit2.position; }
        count += 1u;
    }
    if (count == 2u) {
        segments[id.x] = Segment(first, second, 1u, 0u);
    } else {
        segments[id.x].valid = 0u;
    }
}
"#;

/// Metal counterpart kept alongside WGSL for shader-parity review.
pub const MESH_COMPUTE_MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct PartialRange { float min_value; float max_value; uint valid; uint padding; };
struct Edge { uint a; uint b; };
struct EdgeHit { float4 position; uint valid; uint padding; };

kernel void field_min_max(
    device const float* values [[buffer(0)]],
    device PartialRange* partial_ranges [[buffer(1)]],
    constant uint& value_count [[buffer(6)]],
    uint id [[thread_position_in_grid]],
    uint lane [[thread_index_in_threadgroup]],
    uint group [[threadgroup_position_in_grid]],
    threadgroup float* local_min [[threadgroup(0)]],
    threadgroup float* local_max [[threadgroup(1)]],
    threadgroup uint* local_valid [[threadgroup(2)]]) {
    float minimum = INFINITY;
    float maximum = -INFINITY;
    uint valid = 0;
    for (uint index = id; index < value_count; index += 256) {
        float value = values[index];
        if (!isnan(value) && !isinf(value)) {
            minimum = min(minimum, value);
            maximum = max(maximum, value);
            valid = 1;
        }
    }
    local_min[lane] = minimum;
    local_max[lane] = maximum;
    local_valid[lane] = valid;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    if (lane == 0) {
        float result_min = local_min[0];
        float result_max = local_max[0];
        uint result_valid = local_valid[0];
        for (uint i = 1; i < 256; ++i) {
            result_min = min(result_min, local_min[i]);
            result_max = max(result_max, local_max[i]);
            result_valid = max(result_valid, local_valid[i]);
        }
        partial_ranges[group] = { result_min, result_max, result_valid, 0 };
    }
}

kernel void edge_intersections(
    device const float4* positions [[buffer(2)]],
    device const Edge* edges [[buffer(3)]],
    device EdgeHit* hits [[buffer(4)]],
    device const float* values [[buffer(0)]],
    constant float& level [[buffer(5)]],
    constant uint& edge_count [[buffer(6)]],
    uint id [[thread_position_in_grid]]) {
    if (id >= edge_count) return;
    Edge edge = edges[id];
    float a = values[edge.a];
    float b = values[edge.b];
    if (isnan(a) || isnan(b) || isinf(a) || isinf(b) || (a >= level) == (b >= level)) {
        hits[id].valid = 0;
        return;
    }
    float t = clamp((level - a) / (b - a), 0.0f, 1.0f);
    hits[id] = { float4(mix(positions[edge.a].xyz, positions[edge.b].xyz, t), 0.0f), 1, 0 };
}

struct TriangleEdges { uint e0; uint e1; uint e2; };
struct Segment { float4 start; float4 end; uint valid; uint padding; };

kernel void triangle_segments(
    device const TriangleEdges* triangles [[buffer(6)]],
    device const EdgeHit* edge_hits [[buffer(4)]],
    device Segment* segments [[buffer(7)]],
    constant uint& triangle_count [[buffer(8)]],
    uint id [[thread_position_in_grid]]) {
    if (id >= triangle_count) return;
    TriangleEdges triangle = triangles[id];
    EdgeHit hits[3] = { edge_hits[triangle.e0], edge_hits[triangle.e1], edge_hits[triangle.e2] };
    float4 first = float4(0.0);
    float4 second = float4(0.0);
    uint count = 0;
    for (uint edge = 0; edge < 3; ++edge) {
        if (hits[edge].valid == 0) continue;
        if (count == 0) first = hits[edge].position;
        else if (count == 1) second = hits[edge].position;
        count += 1;
    }
    segments[id] = { first, second, count == 2, 0 };
}
"#;
