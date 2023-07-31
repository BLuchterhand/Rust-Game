struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct ResultBuffer {
    data: array<f32>,
};

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexBuffer {
    data: array<Vertex>, // stride: 32
}

@group(0) @binding(1) 
var<storage, read_write> vertices: VertexBuffer;

@group(0) @binding(2)
var<storage, read_write> result_buffer: ResultBuffer;

@compute
@workgroup_size(1)
fn intersectRayPlane(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    // Access the ray data from the uniform buffer
    let camera_data = camera;

    // Process the ray data (for example, simply returning the origin)
    let result = camera_data.view_pos.xyz.y;
    vertices.data[5].position.z = 13.0;

    // Store the result in the result_buffer
    result_buffer.data[0] = result;
}
