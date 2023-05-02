use std::{error::Error, fmt::Display, iter, mem};

use eyre::Result;
use glam::Vec2;
use ndarray::Axis;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

use crate::fluid::Fluid;

pub struct Renderer {
    pub instance: Instance,
    pub surface: Surface,
    pub surface_config: SurfaceConfiguration,
    pub window: Window,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub pipeline: RenderPipeline,
    pub sampler: Sampler,
    pub bind_group_layout: BindGroupLayout,
    pub quad: Buffer,
}

pub struct FluidTexture {
    pub fluid: Fluid,
    pub texture: Texture,
    pub bind_group: BindGroup,
}

impl FluidTexture {
    pub fn new(fluid: Fluid, renderer: &Renderer) -> Self {
        let texture = renderer.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: fluid.size as u32,
                height: fluid.size as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&Default::default());

        let bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &renderer.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
            ],
        });

        let this = Self {
            fluid,
            texture,
            bind_group,
        };
        this.update(renderer);
        this
    }

    pub fn update(&self, renderer: &Renderer) {
        let densities: Vec<_> = self
            .fluid
            .cells
            .axis_iter(Axis(1))
            .flatten()
            .map(|cell| (cell.density.clamp(0.0, 1.0) * u8::MAX as f32) as u8)
            .collect();

        renderer.queue.write_texture(
            ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&densities),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((mem::size_of::<u8>() * self.fluid.size) as u32),
                rows_per_image: Some(self.fluid.size as u32),
            },
            Extent3d {
                width: self.fluid.size as u32,
                height: self.fluid.size as u32,
                depth_or_array_layers: 1,
            },
        );
    }
}

impl Renderer {
    pub async fn new(window: Window) -> Result<Self> {
        let instance = Instance::new(Default::default());

        let surface = unsafe { instance.create_surface(&window) }?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or(NoAdapter)?;

        let (device, queue) = adapter.request_device(&Default::default(), None).await?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_capabilities.formats[0],
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(include_wgsl!("./shader.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: 2 * mem::size_of::<f32>() as u64,
                    attributes: &vertex_attr_array![0 => Float32x2],
                    step_mode: wgpu::VertexStepMode::Vertex,
                }],
            },
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: Some(BlendState {
                        color: BlendComponent::REPLACE,
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        const VERTICES: &[Vec2] = &[
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
        ];

        let quad = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsages::VERTEX,
        });

        Ok(Self {
            window,
            instance,
            surface,
            surface_config,
            adapter,
            device,
            queue,
            pipeline,
            sampler,
            bind_group_layout,
            quad,
        })
    }

    pub fn render(&self, fluid: &FluidTexture) -> Result<()> {
        let output = self.surface.get_current_texture()?;
        let output_view = output.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &output_view,
                resolve_target: None,
                ops: Operations {
                    load: wgpu::LoadOp::Clear(Default::default()),
                    store: true,
                },
            })],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &fluid.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad.slice(..));
        render_pass.draw(0..6, 0..1);

        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoAdapter;

impl Display for NoAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no compatible wgpu `Adapter`")
    }
}

impl Error for NoAdapter {}
