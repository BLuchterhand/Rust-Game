use opensimplex_noise_rs::OpenSimplexNoise;
use cgmath::*;
use std::{
  collections::HashMap,
};
use std::mem::size_of_val;

use crate::lib::{create_render_pipeline, model};

const CHUNKSIZE: f32 = 16.00;
const NUM_VERTICES: u32 = 17;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkData {
  chunk_size: [u32; 2],
  chunk_corner: [f32; 2],
  min_max_height: [f32; 2],
}

pub struct World {
  terrain: OpenSimplexNoise,
  terrain_scale: f64,
  chunks: HashMap<String, Chunk>
}

pub struct Chunk {
  chunk_size: [u32; 2],
  corner_coord: [f32; 2],
  pub mesh: model::Mesh,
}

impl World {
  pub fn new() -> Self {
    let terrain = OpenSimplexNoise::new(Some(883_279_212_983_182_319));
    let terrain_scale = 0.044;
    let chunks = HashMap::new();

    Self {
      terrain,
      terrain_scale,
      chunks,
    }
  }

  pub fn get_chunk(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, position: Point3<f32>) {
    let y = self.terrain.eval_2d(position.x as f64 * self.terrain_scale, 
      position.z as f64 * self.terrain_scale); // generates value in range (-1, 1)
    
      // get chunk
      let mut x_coord = (position.x / CHUNKSIZE).trunc() * CHUNKSIZE;
      let mut z_coord = (position.z / CHUNKSIZE).trunc() * CHUNKSIZE;

      if x_coord == -0.0 {
        x_coord = 0.0
      }

      if z_coord == -0.0 {
        z_coord = 0.0
      }
      
      let chunk_key = format!("{x_coord},{z_coord}");

      if self.chunks.contains_key(&chunk_key) {
        // load chunk at key
        match self.chunks.get(&chunk_key) {
          None => println!("Chunk not found..."),
          Some(chunk) => {
            println!("Currently in chunk: {:?}", chunk.corner_coord)
          }
        }
      } else {
        // generate new chunk
        println!("Generating new chunk: {:?}", chunk_key);
        self.chunks.insert(format!("{x_coord},{z_coord}"), Chunk::new(
          [x_coord, z_coord], 
          &device,
          &queue,
          chunk_key,
        ));
      };
  }
}

impl Chunk {
  fn new(
    corner_coord: [f32; 2],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    chunk_id: String,
  ) -> Self {
    let min_max_height = (-5.0, 5.0);
    let chunk_size = [16, 16];
    let num_vertices = (chunk_size[0] + 1) * (chunk_size[1] + 1);
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&format!("{}: Vertices", chunk_id.clone())),
      size: (num_vertices * 8 * std::mem::size_of::<f32>() as u32) as _,
      usage: wgpu::BufferUsages::STORAGE
          | wgpu::BufferUsages::VERTEX
          | wgpu::BufferUsages::COPY_SRC,
      mapped_at_creation: false,
    });
    let num_elements = chunk_size[0] * chunk_size[1] * 6;
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&format!("{}: Indices", chunk_id.clone())),
      size: (num_elements * std::mem::size_of::<u32>() as u32) as _,
      usage: wgpu::BufferUsages::STORAGE
          | wgpu::BufferUsages::INDEX
          | wgpu::BufferUsages::COPY_SRC,
      mapped_at_creation: false,
    });
    let new_string = chunk_id.clone();
    let mesh = model::Mesh {
      name: new_string,
      vertex_buffer,
      index_buffer,
      num_elements,
      material: 0,
      index_format: wgpu::IndexFormat::Uint32,
    };

    let chunk = Self {
      chunk_size,
      corner_coord,
      mesh,
    };

    let data = ChunkData {
      chunk_size: chunk.chunk_size,
      chunk_corner: chunk.corner_coord,
      min_max_height: [-5.0, 5.0],
    };

    let gen_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("WorldPipeline: ChunkData"),
      size: size_of_val(&chunk) as _,
      usage: wgpu::BufferUsages::MAP_READ
          // | wgpu::BufferUsages::UNIFORM
          | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    queue.write_buffer(&gen_buffer, 0, bytemuck::bytes_of(&data));

    chunk

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
      let render_pipeline = crate::lib::create_render_pipeline(
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
      world: &'a World,
      camera_bind_group: &'a wgpu::BindGroup,
      light_bind_group: &'a wgpu::BindGroup,
  ) {
      render_pass.set_pipeline(&self.render_pipeline);
      render_pass.set_bind_group(0, camera_bind_group, &[]);
      render_pass.set_bind_group(1, light_bind_group, &[]);
      for chunk in &world.chunks {
          render_pass.set_index_buffer(chunk.1.mesh.index_buffer.slice(..), chunk.1.mesh.index_format);
          render_pass.set_vertex_buffer(0, chunk.1.mesh.vertex_buffer.slice(..));
          render_pass.draw_indexed(0..chunk.1.mesh.num_elements, 0, 0..1);
      }
  }
}

pub struct WorldHackPipeline {
  texture_size: u32,
  gen_layout: wgpu::BindGroupLayout,
  gen_pipeline: wgpu::RenderPipeline,
  render_pipeline: wgpu::RenderPipeline,
  chunk_size: cgmath::Vector2<u32>,
  min_max_height: cgmath::Vector2<f32>,
}

impl WorldHackPipeline {
  pub fn new(
      device: &wgpu::Device,
      chunk_size: cgmath::Vector2<u32>,
      min_max_height: cgmath::Vector2<f32>,
      camera_layout: &wgpu::BindGroupLayout,
      light_layout: &wgpu::BindGroupLayout,
      color_format: wgpu::TextureFormat,
      depth_format: Option<wgpu::TextureFormat>,
  ) -> Self {
      // Given that the vertices in the chunk are 2 vec3s, num_indices should = num_vertices
      let texture_size = 512;

      let gen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          label: Some("HackTerrainPipeline::BindGroupLayout"),
          entries: &[wgpu::BindGroupLayoutEntry {
              binding: 0,
              visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
              ty: wgpu::BindingType::Buffer {
                  ty: wgpu::BufferBindingType::Uniform,
                  has_dynamic_offset: false,
                  min_binding_size: None,
              },
              count: None,
          }],
      });

      let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("HackTerrainPipeline::PipelineLayout"),
          bind_group_layouts: &[&gen_layout],
          push_constant_ranges: &[],
      });

      let shader = device.create_shader_module(wgpu::include_wgsl!("terrain.wgsl"));
      let gen_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("HackTerrainPipeline::GenPipeline"),
          layout: Some(&pipeline_layout),
          vertex: wgpu::VertexState {
              module: &shader,
              entry_point: "gen_terrain_vertex",
              buffers: &[],
          },
          primitive: wgpu::PrimitiveState {
              topology: wgpu::PrimitiveTopology::TriangleList,
              cull_mode: None,
              ..Default::default()
          },
          depth_stencil: None,
          multisample: wgpu::MultisampleState {
              count: 1,
              mask: !0,
              alpha_to_coverage_enabled: false,
          },
          fragment: Some(wgpu::FragmentState {
              module: &shader,
              entry_point: "gen_terrain_fragment",
              targets: &[
                  Some(wgpu::ColorTargetState {
                      format: wgpu::TextureFormat::R32Uint,
                      blend: None,
                      write_mask: wgpu::ColorWrites::ALL,
                  }),
                  Some(wgpu::ColorTargetState {
                      format: wgpu::TextureFormat::R32Uint,
                      blend: None,
                      write_mask: wgpu::ColorWrites::ALL,
                  }),
              ],
          }),
          multiview: None,
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
              array_stride: 24,
              step_mode: wgpu::VertexStepMode::Vertex,
              attributes: &[
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float32x3,
                      offset: 0,
                      shader_location: 0,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float32x3,
                      offset: 12,
                      shader_location: 1,
                  },
              ],
          }],
          &shader,
      );

      Self {
          chunk_size,
          min_max_height,
          texture_size,
          gen_layout,
          gen_pipeline,
          render_pipeline,
      }
  }

  pub fn render<'a, 'b>(
      &'a self,
      render_pass: &'b mut wgpu::RenderPass<'a>,
      world: &'a World,
      camera_bind_group: &'a wgpu::BindGroup,
      light_bind_group: &'a wgpu::BindGroup,
  ) {
      render_pass.set_pipeline(&self.render_pipeline);
      render_pass.set_bind_group(0, camera_bind_group, &[]);
      render_pass.set_bind_group(1, light_bind_group, &[]);
      for chunk in &world.chunks {
          render_pass
              .set_index_buffer(chunk.1.mesh.index_buffer.slice(..), chunk.1.mesh.index_format);
          render_pass.set_vertex_buffer(0, chunk.1.mesh.vertex_buffer.slice(..));
          render_pass.draw_indexed(0..chunk.1.mesh.num_elements, 0, 0..1);
      }
  }

  fn create_work_texture(&self, device: &wgpu::Device, index: bool) -> wgpu::Texture {
      device.create_texture(&wgpu::TextureDescriptor {
          label: Some(if index {
              "Index Texture"
          } else {
              "Vertex Texture"
          }),
          size: wgpu::Extent3d {
              width: self.texture_size,
              height: self.texture_size,
              depth_or_array_layers: 1,
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: wgpu::TextureFormat::R32Uint,
          usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
      })
  }
}
