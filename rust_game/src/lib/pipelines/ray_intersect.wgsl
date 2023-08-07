struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

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


struct IndexBuffer {
    data: array<u32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read_write> vertices: VertexBuffer;
@group(0) @binding(2) var<storage, read_write> indices: IndexBuffer;
@group(0) @binding(3) var<storage, read_write> result_buffer: ResultBuffer;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

fn intersectRayWithTriangle(ray: Ray, v0: vec3<f32>, v1: vec3<f32>, v2: vec3<f32>) -> f32 {
  let edge1 = v1 - v0;
  let edge2 = v2 - v0;
  let h = cross(ray.direction, edge2);
  let a = dot(edge1, h);

  if (a > -0.00001 && a < 0.00001) {
    return -1.0; // Ray is parallel to the triangle
  }

  let f = 1.0 / a;
  let s = ray.origin - v0;
  let u = f * dot(s, h);

  if (u < 0.0 || u > 1.0) {
    return -1.0;
  }

  let q = cross(s, edge1);
  let v = f * dot(ray.direction, q);

  if (v < 0.0 || u + v > 1.0) {
    return -1.0;
  }

  let t = f * dot(edge2, q);
  return t;
}

@compute
@workgroup_size(1)
fn intersectRayPlane() {
  let ray = Ray(vec3<f32>(camera.view_pos.xyz), vec3<f32>(0.0, -1.0, 0.0));
  var closestDistance: f32 = 1.0e38;

  for (var i: u32 = 0u; i < 6144u; i = i + 6u) {
    let v00 = vertices.data[indices.data[i]].position;
    let v01 = vertices.data[indices.data[i + 1u]].position;
    let v11 = vertices.data[indices.data[i + 2u]].position;

    let t1 = intersectRayWithTriangle(ray, v00, v01, v11);

    let v10 = vertices.data[indices.data[i + 5u]].position;

    let t2 = intersectRayWithTriangle(ray, v00, v11, v10);

    if (t1 > 0.0) {
      closestDistance = min(closestDistance, t1);
    }

    if (t2 > 0.0) {
      closestDistance = min(closestDistance, t2);
    }
  }

  result_buffer.data[0] = closestDistance;
}
