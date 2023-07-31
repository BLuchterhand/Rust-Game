use crate::world::{World, Chunk};
use crate::lib::camera::Camera;


pub struct RayIntersectPipeline {
  pub compute_pipeline: wgpu::ComputePipeline,
  pub compute_bind_group: wgpu::BindGroup,
  pub result_buffer: wgpu::Buffer,
  pub staging_buffer: wgpu::Buffer,
  pub size: u64,
}

impl RayIntersectPipeline {
    pub fn new(
        device: &wgpu::Device,
        camera: &Camera,
        camera_buffer: &wgpu::Buffer,
        world: &World,
    ) -> Self {
        let compute_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("compute_layout"),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_layout],
                push_constant_ranges: &[],
        });

        let compute_pipeline = {
            let desc = wgpu::ShaderModuleDescriptor {
                label: Some("Compute"),
                source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
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

        let num_elements = 32 * 32 * 6;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (num_elements * std::mem::size_of::<u32>() as u32) as _,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: index_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: result_buffer.as_entire_binding(),
            }],
            label: Some("compute_bind_group"),
        });

        Self {
            compute_pipeline,
            compute_bind_group,
            result_buffer,
            staging_buffer,
            size,
        }
    }

    pub async fn get_buffer_contents(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk: Chunk,
    ) -> f32 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
        });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Ray Intersection: ComputePass"),
        });
        cpass.set_pipeline(&self.compute_pipeline);
        cpass.set_bind_group(0, &self.compute_bind_group, &[]);
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
            self.size,
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
            &self.staging_buffer.unmap();

            return result[0]
        } else {
            panic!("failed to run compute on gpu!")
        }
    }
}
