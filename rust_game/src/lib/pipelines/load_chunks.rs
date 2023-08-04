use std::{
  collections::HashMap,
  mem::size_of_val,
};

use cgmath::Vector2;

use crate::lib::model;

struct VertexData {
    position: [f32; 3],
    normal: [f32; 3],
}

#[derive(Clone)]
pub struct RawBufferData {
    pub vertex_data: [u8; 34848],
    pub index_data: [u8; 24576],
}

pub struct Chunk {
  pub mesh: model::Mesh,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkData {
    chunk_size: [u32; 2],
    chunk_corner: [i32; 2],
    min_max_height: [f32; 2],
}

pub struct ComputeWorld {
  pub chunks: HashMap<String, RawBufferData>,
}

impl ComputeWorld {
  pub fn new() -> Self {
      Self {
          chunks: HashMap::new(),
      }
  }

  pub async fn load_chunks(
      &mut self,
      device: &wgpu::Device,
      queue: &wgpu::Queue,
      pipeline: &ComputeWorldPipeline,
      anchor_coords: HashMap<String, Vec<i32>>
  ) {
        let mut new_chunks = HashMap::new();
        for (chunk_key, anchor_coords) in anchor_coords.iter(){
            if let Some(chunk) = self.chunks.remove(chunk_key) {
                // chunk exists
                new_chunks.insert(chunk_key.clone(), chunk);
            } else {
                // chunk does not exist, generate
                let new_chunk = pipeline.gen_chunk(&device, &queue, Vector2::new(
                    anchor_coords[0], anchor_coords[1]
                ));
                let num_vertices = (32 + 1) * (32 + 1);
                let num_indices = (32) * (32);
                let vertex_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Vertices"),
                    size: (num_vertices * 8 * std::mem::size_of::<f32>() as u32) as _,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let index_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Indices"),
                    size: (num_indices * 6 * std::mem::size_of::<u32>() as u32) as _,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                encoder.copy_buffer_to_buffer(
                    &new_chunk.mesh.vertex_buffer, 
                    0, 
                    &vertex_staging_buffer, 
                    0, 
                    (num_vertices * 8 * std::mem::size_of::<f32>() as u32) as _,
                );
                queue.submit(Some(encoder.finish()));

                let buffer_slice = vertex_staging_buffer.slice(..);
                let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
                device.poll(wgpu::Maintain::Wait);

                let mut vertex_data: [u8; 34848] = [0; 34848];
                if let Some(Ok(())) = receiver.receive().await {
                    let data = buffer_slice.get_mapped_range();
                    vertex_data.copy_from_slice(bytemuck::cast_slice(&data));

                    // let vertex_count = data.len() / 8 / std::mem::size_of::<f32>(); // 2 attributes (position and normal)
                    // for i in 0..vertex_count {
                    //     let vertex_offset = i * std::mem::size_of::<VertexData>();

                    //     let position_bytes = &data[vertex_offset..vertex_offset + 3 * std::mem::size_of::<f32>()];
                    //     let result: Vec<f32> = bytemuck::cast_slice(&position_bytes).to_vec();
                    //     new_chunks.insert(chunk_key.clone(), RawBufferData {
                    //         vertex_data: result,
                    //     });

                    //     if chunk_key == "0_0"{
                    //         println!("HERE");
                    //         println!("{:?}", chunk_key);
                    //     }
                    // }
                    drop(data);
                    vertex_staging_buffer.unmap();
                } else {
                    panic!("failed to ingest vertex data!")
                }

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                encoder.copy_buffer_to_buffer(
                    &new_chunk.mesh.index_buffer, 
                    0, 
                    &index_staging_buffer, 
                    0, 
                    (num_indices * 6 * std::mem::size_of::<u32>() as u32) as _,
                );
                queue.submit(Some(encoder.finish()));

                let buffer_slice = index_staging_buffer.slice(..);
                let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
                device.poll(wgpu::Maintain::Wait);

                let mut index_data: [u8; 24576] = [0; 24576];
                if let Some(Ok(())) = receiver.receive().await {
                    let data = buffer_slice.get_mapped_range();
                    index_data.copy_from_slice(bytemuck::cast_slice(&data));
                    drop(data);
                    index_staging_buffer.unmap();
                } else {
                    panic!("failed to ingest index data!")
                }
                
                new_chunks.insert(chunk_key.clone(), RawBufferData {
                    vertex_data: vertex_data.clone(),
                    index_data: index_data.clone(),
                });
            }
        }

      self.chunks = new_chunks;
  }
}

pub struct ComputeWorldPipeline {
  chunk_size: cgmath::Vector2<u32>,
  min_max_height: cgmath::Vector2<f32>,
  gen_layout: wgpu::BindGroupLayout,
  gen_pipeline: wgpu::ComputePipeline,
}

impl ComputeWorldPipeline {
  pub fn new(
      device: &wgpu::Device,
      chunk_size: cgmath::Vector2<u32>,
      min_max_height: cgmath::Vector2<f32>,
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

      Self {
          chunk_size,
          min_max_height,
          gen_layout,
          gen_pipeline,
      }
  }
  
  pub fn gen_chunk(
      &self,
      device: &wgpu::Device,
      queue: &wgpu::Queue,
      corner: cgmath::Vector2<i32>,
  ) -> Chunk {
      let chunk_name = format!("Chunk {:?}", corner);
      let num_vertices = (self.chunk_size.x + 1) * (self.chunk_size.y + 1);
      let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
          label: Some(&format!("{}: Vertices", chunk_name)),
          size: (num_vertices * 8 * std::mem::size_of::<f32>() as u32) as _,
          usage: wgpu::BufferUsages::STORAGE
              | wgpu::BufferUsages::VERTEX
              | wgpu::BufferUsages::COPY_SRC,
          mapped_at_creation: false,
      });
      let num_elements = self.chunk_size.x * self.chunk_size.y * 6;
      let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
          label: Some(&format!("{}: Indices", chunk_name)),
          size: (num_elements * std::mem::size_of::<u32>() as u32) as _,
          usage: wgpu::BufferUsages::STORAGE
              | wgpu::BufferUsages::INDEX
              | wgpu::BufferUsages::COPY_SRC,
          mapped_at_creation: false,
      });
      let chunk = Chunk {
          mesh: model::Mesh {
              name: chunk_name,
              vertex_buffer,
              index_buffer,
              num_elements,
              material: 0,
              index_format: wgpu::IndexFormat::Uint32,
          },
      };

      let data = ChunkData {
          chunk_size: self.chunk_size.into(),
          chunk_corner: corner.into(),
          min_max_height: self.min_max_height.into(),
      };
      let gen_buffer = device.create_buffer(&wgpu::BufferDescriptor {
          label: Some("TerrainPipeline: ChunkData"),
          size: size_of_val(&data) as _,
          usage: wgpu::BufferUsages::UNIFORM
              | wgpu::BufferUsages::COPY_DST,
          mapped_at_creation: false,
      });
      queue.write_buffer(&gen_buffer, 0, bytemuck::bytes_of(&data));

      let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
          label: Some("TerrainPipeline: BindGroup"),
          layout: &self.gen_layout,
          entries: &[
              wgpu::BindGroupEntry {
                  binding: 0,
                  resource: gen_buffer.as_entire_binding(),
              },
              wgpu::BindGroupEntry {
                  binding: 1,
                  resource: chunk.mesh.vertex_buffer.as_entire_binding(),
              },
              wgpu::BindGroupEntry {
                  binding: 2,
                  resource: chunk.mesh.index_buffer.as_entire_binding(),
              },
          ],
      });

      let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("TerrainPipeline::gen_chunk"),
      });

      let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
          label: Some("TerrainPipeline: ComputePass"),
      });
      cpass.set_pipeline(&self.gen_pipeline);
      cpass.set_bind_group(0, &bind_group, &[]);
      cpass.dispatch_workgroups(
          (((self.chunk_size.x + 1) * (self.chunk_size.y + 1)) as f32 / 64.0).ceil() as _,
          1,
          1,
      );
      drop(cpass);

      queue.submit(std::iter::once(encoder.finish()));
      device.poll(wgpu::Maintain::Wait);

      chunk
  }
}