use std::collections::HashMap;
use std::sync::Arc;

use cgmath::Vector2;
use wgpu::util::DeviceExt;

use crate::lib::model::Mesh;
use crate::lib::pipelines::load_chunks::{Chunk, RawBufferData};
use crate::lib::create_render_pipeline;


#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkData {
    chunk_size: [u32; 2],
    chunk_corner: [i32; 2],
    min_max_height: [f32; 2],
}

pub struct World {
    pub chunks: HashMap<String, Chunk>,
    pub requested_chunks: HashMap<String, Vec<i32>>,
    chunk_size: cgmath::Vector2<u32>,
    pub raw_buffer_data: HashMap<String, RawBufferData>,
}

impl World {
    pub fn new(chunk_size: cgmath::Vector2<u32>) -> Self {
        Self {
            chunks: HashMap::new(),
            requested_chunks: HashMap::new(),
            chunk_size,
            raw_buffer_data: HashMap::new(),
        }
    }

    pub fn load_chunks<'a, 'b>(
        &mut self,
        position: cgmath::Vector3<f32>,
    ) {
        // define chunk boundaries
        let r = 10; // chunk distance
        let n = 2 * r + 1;
        let mut x: i32;
        let mut z: i32;

        // Get the x and z coords of the chunk identifier
        let x_coord =
            ((position.x as i32 / self.chunk_size.x as i32) + r) * self.chunk_size.x as i32;
        let z_coord =
            ((position.z as i32 / self.chunk_size.y as i32) + r) * self.chunk_size.y as i32;

        let mut new_chunks = HashMap::new();

        for i in 0..n {
            for j in 0..n {
                x = i - r;
                z = j - r;

                // convert anchor point to coordinates
                let x_anchor = x * self.chunk_size.x as i32 + x_coord - (self.chunk_size.x as i32 * r);
                let z_anchor = z * self.chunk_size.y as i32 + z_coord - (self.chunk_size.y as i32 * r);
                let anchor_coords = vec![x_anchor, z_anchor];

                // if chunk is within render distance
                if x * x + z * z <= r * r + 1 {
                    let chunk_key = format!("{}_{}", x_anchor, z_anchor);
                    if let Some(chunk) = self.chunks.remove(&chunk_key) {
                        // generated chunk exists, keep it
                        new_chunks.insert(chunk_key.clone(), chunk);
                        if let Some(_) = self.requested_chunks.remove(&chunk_key) {
                            // leave chunk removed from requested list
                        }
                    } else {
                        // generated chunk does not exist, request it
                        println!("DOESNT EXIST");
                        if let Some(coords) = self.requested_chunks.remove(&chunk_key) {
                            // chunk exists
                            self.requested_chunks.insert(chunk_key.clone(), coords);
                        } else {
                            // chunk does not exist, request
                            self.requested_chunks.insert(chunk_key.clone(), anchor_coords);
                        }
                    }
                }
            }
        }
  
        self.chunks = new_chunks;
    }

    pub fn ingest_chunk_data(&mut self, device: &wgpu::Device) {
        for (chunk_key, chunk_data) in &self.raw_buffer_data {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: &chunk_data.vertex_data,
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: &chunk_data.index_data,
                usage: wgpu::BufferUsages::INDEX,
            });

            // chunk size x * chunk size y * 6
            let num_elements = 32 * 32 * 6;
            let chunk = Chunk {
                mesh: Mesh {
                    name: chunk_key.to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements,
                    material: 0,
                    index_format: wgpu::IndexFormat::Uint32,
                },
            };

            self.chunks.insert(chunk_key.to_string(), chunk);
        }
        self.raw_buffer_data = HashMap::new();
    }
}

pub struct WorldPipeline {
    chunk_size: cgmath::Vector2<u32>,
    min_max_height: cgmath::Vector2<f32>,
    gen_layout: wgpu::BindGroupLayout,
    gen_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
}

impl WorldPipeline {
    pub fn new(
        device: &wgpu::Device,
        chunk_size: cgmath::Vector2<u32>,
        min_max_height: cgmath::Vector2<f32>,
        camera_layout: &wgpu::BindGroupLayout,
        light_layout: &wgpu::BindGroupLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let gen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ChunkLoader::Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("terrain.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("TerrainPipeline::Gen::PipelineLayout"),
            bind_group_layouts: &[&gen_layout],
            push_constant_ranges: &[],
        });
        let gen_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("TerrainPipeline::ComputePipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "gen_terrain_compute",
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("TerrainPipeline::Render::PipelineLayout"),
                bind_group_layouts: &[camera_layout, light_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = create_render_pipeline(
            device,
            &render_pipeline_layout,
            color_format,
            depth_format,
            &[wgpu::VertexBufferLayout {
                array_stride: 32,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 16,
                        shader_location: 1,
                    },
                ],
            }],
            &shader,
        );

        Self {
            chunk_size,
            min_max_height,
            gen_layout,
            gen_pipeline,
            render_pipeline,
        }
    }

    pub fn render<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        terrain: &'a World,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, light_bind_group, &[]);
        for (_, chunk) in &terrain.chunks {
            render_pass
                .set_index_buffer(chunk.mesh.index_buffer.slice(..), chunk.mesh.index_format);
            render_pass.set_vertex_buffer(0, chunk.mesh.vertex_buffer.slice(..));
            render_pass.draw_indexed(0..chunk.mesh.num_elements, 0, 0..1);
        }
    }
}
