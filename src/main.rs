use sdl2::event::{Event, EventType, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::video::FullscreenType;
use std::borrow::Cow;

use pollster;

use wgpu::util::DeviceExt;
use wgpu::SurfaceError;

const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 900;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: glam::Vec3,
    color: glam::Vec3,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let triangle_base: Vec<Vertex> = vec![
        Vertex {
            position: glam::Vec3::new(0.0, 0.5, 0.0),
            color: glam::Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: glam::Vec3::new(-0.5, -0.5, 0.0),
            color: glam::Vec3::new(0.0, 1.0, 0.0),
        },
        Vertex {
            position: glam::Vec3::new(0.5, -0.5, 0.0),
            color: glam::Vec3::new(0.0, 0.0, 1.0),
        },
    ];

    let mut triangle = triangle_base.clone();

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    video_subsystem.text_input().stop();
    let event_subsystem = sdl_context.event().unwrap();

    let mut window = video_subsystem
        .window("Window", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let (width, height) = window.size();

    let mouse_util = sdl_context.mouse();
    // uncomment to see the issue improve, but not disappear
    // mouse_util.set_relative_mouse_mode(true);

    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let adapter_opt = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }));

    let adapter = match adapter_opt {
        Some(a) => a,
        None => panic!("No suitable adapter found."),
    };

    let (device, queue) = match pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            limits: wgpu::Limits::default(),
            label: Some("device"),
            features: wgpu::Features::empty(),
        },
        None,
    )) {
        Ok(a) => a,
        Err(_e) => panic!("Could not get requested device."),
    };

    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shader.wgsl"))),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
        ],
    };

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[vertex_buffer_layout],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            }],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_preferred_format(&adapter).unwrap(),
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &config);

    let mut vertex_buffer;

    let mut angle: f32 = 0.0;

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        let events: Vec<Event> = event_pump.poll_iter().collect();

        for event in events {
            match event {
                Event::MouseMotion { x, y, .. } => {
                    // comment this, and the triangle movement becomes jittery
                    println!("x: {x}, y: {y}");
                }
                Event::Window { win_event, .. } => {
                    if let WindowEvent::Resized(w, h) = win_event {
                        config.width = w as u32;
                        config.height = h as u32;
                        surface.configure(&device, &config);
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F11),
                    ..
                } => {
                    let fs = window.fullscreen_state();
                    match fs {
                        FullscreenType::Off => {
                            window.set_fullscreen(FullscreenType::Desktop)?;
                            let (width, height) = window.size();
                            config.width = width;
                            config.height = height;
                            surface.configure(&device, &config);
                        }
                        FullscreenType::Desktop => {
                            window.set_fullscreen(FullscreenType::Off)?;
                            let (width, height) = window.size();
                            config.width = width;
                            config.height = height;
                            surface.configure(&device, &config);
                        }
                        _ => window.set_fullscreen(FullscreenType::Off)?,
                    }
                }
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                _ => {}
            }
        }

        angle += 0.02;

        for (index, vert_base) in triangle_base.iter().enumerate() {
            let quat = glam::Quat::from_rotation_z(f32::sin(angle));
            let new_pos = quat.mul_vec3(vert_base.position);
            triangle[index].position = new_pos;
        }

        vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bullet Vertex Buffer"),
            contents: bytemuck::cast_slice(triangle.as_slice()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command_encoder"),
        });

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                let reason = match err {
                    SurfaceError::Timeout => "Timeout",
                    SurfaceError::Outdated => "Outdated",
                    SurfaceError::Lost => "Lost",
                    SurfaceError::OutOfMemory => "OutOfMemory",
                };
                panic!("Failed to get current surface texture! Reason: {}", reason)
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.03,
                            g: 0.03,
                            b: 0.03,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&render_pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
        frame.present();
    }
    Ok(())
}
