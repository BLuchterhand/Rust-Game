use cgmath::Vector2;

use crate::world::{World};
use crate::lib::pipelines::load_chunks::Chunk;
use crate::lib::camera::Camera;

use super::load_chunks::RawBufferData;


pub struct RayIntersectPipeline {
  pub ray_intersect_pipeline: wgpu::ComputePipeline,
  pub bind_group: wgpu::BindGroup,
  pub index_buffer: wgpu::Buffer,
  pub vertex_buffer: wgpu::Buffer,
  pub result_buffer: wgpu::Buffer,
  pub staging_buffer: wgpu::Buffer,
  pub buffer_size: u64,
}

impl RayIntersectPipeline {
    pub fn new(
        device: &wgpu::Device,
        camera: &Camera,
        camera_buffer: &wgpu::Buffer,
        world: &World,
        chunk_size: Vector2<u32>
    ) -> Self {
        let compute_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Camera buffer
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
                    // Vertex buffer
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
                    // Index buffer
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
                    // Result buffer
                    wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                    },
                ],
                label: Some("compute_layout"),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_layout],
                push_constant_ranges: &[],
        });

        let ray_intersect_pipeline = {
            let desc = wgpu::ShaderModuleDescriptor {
                label: Some("Compute"),
                source: wgpu::ShaderSource::Wgsl(include_str!("ray_intersect.wgsl").into()),
            };

            let shader = device.create_shader_module(desc);
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{:?}", shader)),
                layout: Some(&compute_pipeline_layout),
                module: &shader,
                entry_point: &"intersectRayPlane",
            })
        };
        
        let size = std::mem::size_of::<f32>() as wgpu::BufferAddress;
        let result_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let num_elements = chunk_size.x * chunk_size.y * 6;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (num_elements * std::mem::size_of::<u32>() as u32) as _,
            usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let num_vertices = (chunk_size.x + 1) * (chunk_size.y + 1);
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (num_vertices * 8 * std::mem::size_of::<f32>() as u32) as _,
            usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: result_buffer.as_entire_binding(),
                }
            ],
            label: Some("compute_bind_group"),
        });

        Self {
            ray_intersect_pipeline,
            bind_group,
            index_buffer,
            vertex_buffer,
            result_buffer,
            staging_buffer,
            buffer_size: size,
        }
    }

    pub async fn ray_intersect(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        raw_buffer_data: RawBufferData,
    ) -> f32 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
        });

        queue.write_buffer(&self.vertex_buffer, 0, &raw_buffer_data.vertex_data);
        queue.write_buffer(&self.index_buffer, 0, &raw_buffer_data.index_data);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Ray Intersection: ComputePass"),
        });
        cpass.set_pipeline(&self.ray_intersect_pipeline);
        cpass.set_bind_group(0, &self.bind_group, &[]);
        cpass.dispatch_workgroups(
            1,
            1,
            1,
        );
        drop(cpass);
        queue.submit(std::iter::once(encoder.finish()));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        encoder.copy_buffer_to_buffer(
            &self.result_buffer, 
            0, 
            &self.staging_buffer, 
            0, 
            self.buffer_size,
        );
        queue.submit(Some(encoder.finish()));

        let buffer_slice = &self.staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        device.poll(wgpu::Maintain::Wait);

        if let Some(Ok(())) = receiver.receive().await {
            let data = buffer_slice.get_mapped_range();

            let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
            drop(data);
            let _ = &self.staging_buffer.unmap();

            return result[0]
        } else {
            panic!("failed to run compute on gpu!")
        }
    }
}
