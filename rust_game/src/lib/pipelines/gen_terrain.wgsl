// ============================
// Terrain Generation
// ============================

// https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket
// - Less condensed glsl implementation with comments can be found at https://weber.itn.liu.se/~stegu/jgt2012/article.pdf

fn permute3(x: vec3<f32>) -> vec3<f32> { return (((x * 34.) + 1.) * x) % vec3<f32>(289.); }

fn snoise2(v: vec2<f32>) -> f32 {
    let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    var i: vec2<f32> = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);
    // I flipped the condition here from > to < as it fixed some artifacting I was observing
    var i1: vec2<f32> = select(vec2<f32>(1., 0.), vec2<f32>(0., 1.), (x0.x < x0.y));
    var x12: vec4<f32> = x0.xyxy + C.xxzz - vec4<f32>(i1, 0., 0.);
    i = i % vec2<f32>(289.);
    let p = permute3(permute3(i.y + vec3<f32>(0., i1.y, 1.)) + i.x + vec3<f32>(0., i1.x, 1.));
    var m: vec3<f32> = max(0.5 - vec3<f32>(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3<f32>(0.));
    m = m * m;
    m = m * m;
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;
    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));
    let g = vec3<f32>(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return 130. * dot(m, g);
}


fn fbm(p: vec2<f32>) -> f32 {
    let NUM_OCTAVES: u32 = 5u;
    var x = p * 0.01;
    var v = 0.0;
    var a = 0.5;
    let shift = vec2<f32>(100.0);
    let cs = vec2<f32>(cos(0.5), sin(0.5));
    let rot = mat2x2<f32>(cs.x, cs.y, -cs.y, cs.x);

    for (var i = 0u; i < NUM_OCTAVES; i = i + 1u) {
        v = v + a * snoise2(x);
        x = rot * x * 2.0 + shift;
        a = a * 0.5;
    }

    return v;
}

struct ChunkData {
    chunk_size: vec2<u32>,
    chunk_corner: vec2<i32>,
    min_max_height: vec2<f32>,
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexBuffer {
    data: array<Vertex>, // stride: 32
}

struct IndexBuffer {
    data: array<u32>,
}

@group(0) @binding(0) var<uniform> chunk_data: ChunkData;
@group(0) @binding(1) var<storage, read_write> vertices: VertexBuffer;
@group(0) @binding(2) var<storage, read_write> indices: IndexBuffer;

fn terrain_point(p: vec2<f32>, min_max_height: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(
        p.x,
        mix(min_max_height.x, min_max_height.y, fbm(p)),
        p.y,
    );
}

fn terrain_vertex(p: vec2<f32>, min_max_height: vec2<f32>) -> Vertex {
    let v = terrain_point(p, min_max_height);

    let tpx = terrain_point(p + vec2<f32>(0.1, 0.0), min_max_height) - v;
    let tpz = terrain_point(p + vec2<f32>(0.0, 0.1), min_max_height) - v;
    let tnx = terrain_point(p + vec2<f32>(-0.1, 0.0), min_max_height) - v;
    let tnz = terrain_point(p + vec2<f32>(0.0, -0.1), min_max_height) - v;

    let pn = normalize(cross(tpz, tpx));
    let nn = normalize(cross(tnz, tnx));

    let n = (pn + nn) * 0.5;

    return Vertex(v, n);
}

fn index_to_p(vert_index: u32, chunk_size: vec2<u32>, chunk_corner: vec2<i32>) -> vec2<f32> {
    return vec2(
        f32(vert_index) % f32(chunk_size.x + 1u),
        f32(vert_index / (chunk_size.x + 1u)),
    ) + vec2<f32>(chunk_corner);
}

@compute @workgroup_size(64)
fn gen_terrain_compute(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    // Create vert_component
    let vert_index = gid.x;

    let p = index_to_p(vert_index, chunk_data.chunk_size, chunk_data.chunk_corner);

    vertices.data[vert_index] = terrain_vertex(p, chunk_data.min_max_height);

    // Create indices
    let start_index = gid.x * 6u; // using TriangleList

    if (start_index >= (chunk_data.chunk_size.x * chunk_data.chunk_size.y * 6u)) { return; }

    let v00 = vert_index + gid.x / chunk_data.chunk_size.x;
    let v10 = v00 + 1u;
    let v01 = v00 + chunk_data.chunk_size.x + 1u;
    let v11 = v01 + 1u;

    indices.data[start_index] = v00;
    indices.data[start_index + 1u] = v01;
    indices.data[start_index + 2u] = v11;
    indices.data[start_index + 3u] = v00;
    indices.data[start_index + 4u] = v11;
    indices.data[start_index + 5u] = v10;
}
