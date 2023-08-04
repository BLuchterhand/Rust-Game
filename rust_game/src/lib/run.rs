use std::{sync::{Arc, Mutex}, collections::HashMap, time::Instant, thread};

use instant::Duration;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, WindowBuilder},
};

use crate::lib::{State, pipelines::load_chunks::{ComputeWorldPipeline, ComputeWorld, RawBufferData}};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_cursor_visible(false);
    // TODO: Fix this (wrap in some?)
    window
        .set_cursor_grab(CursorGrabMode::Confined)
        .or_else(|_e| window.set_cursor_grab(CursorGrabMode::Locked));
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .with_visible(false)
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut last_render_time = instant::Instant::now();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
    });

    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
            },
            None, // Trace path
        )
        .await
        .unwrap();

    let compute_device = Arc::new(device);
    let compute_queue = Arc::new(queue);
    let render_device = Arc::clone(&compute_device);
    let render_queue = Arc::clone(&compute_queue);
    let mut state = State::new(
        window,
        surface,
        adapter,
        render_device,
        render_queue,
    ).await;
    state.window().set_visible(true);

    let camera_bind_group_layout =
            compute_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("camera_bind_group_layout"),
            });

    let chunk_size = (32, 32).into();
    let world_chunks: Arc<Mutex<HashMap<String, RawBufferData>>> = Arc::new(Mutex::new(HashMap::new()));
    let world_chunks_shared = Arc::clone(&world_chunks);
    let mut world_compute = ComputeWorld::new();
    let min_max_height = (-5.0, 5.0).into();
    let world_pipeline = ComputeWorldPipeline::new(
        &compute_device,
        chunk_size,
        min_max_height,
    );

    let requested_chunks: Arc<Mutex<HashMap<String, Vec<i32>>>> = Arc::new(Mutex::new(HashMap::new()));
    let requested_chunks_shared = Arc::clone(&requested_chunks);

    tokio::spawn(async move {
        let mut last_execution_time = Instant::now();

        loop {
            let now = Instant::now();
            if now - last_execution_time >= Duration::from_millis(500) {
                let mut temp_requested_chunks = HashMap::new();
                if let Ok(x) = requested_chunks.lock() {
                    temp_requested_chunks = x.clone();
                }

                world_compute.load_chunks(
                    &compute_device, 
                    &compute_queue, 
                    &world_pipeline,
                    temp_requested_chunks,
                ).await;

                if let Ok(mut x) = world_chunks_shared.lock() {
                    x.extend(world_compute.chunks.clone());
                }

                last_execution_time = now;
            }

            thread::sleep(Duration::from_millis(500));
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => state.window().request_redraw(),
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1)
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() && !state.input(event) => {
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                if let Ok(mut x) = world_chunks.lock() {
                    if x.len() > 0 {
                        state.world.raw_buffer_data = HashMap::new();
                        state.world.raw_buffer_data.extend(x.drain());
                    }
                }

                state.world.load_chunks(
                    (
                        state.camera.position.x,
                        state.camera.position.y,
                        state.camera.position.z,
                    ).into(),
                );

                if let Ok(mut x) = requested_chunks_shared.lock() {
                    *x = state.world.requested_chunks.clone();
                }

                let now = instant::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                futures::executor::block_on(state.update(dt));
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            _ => {}
        }
    });
}
