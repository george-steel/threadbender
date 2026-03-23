use core::num;
use std::num::NonZero;

use bytemuck::bytes_of;
use clothoid::spline::ClothoidSegParams;
use glam::{Affine2, UVec2, Vec2};
use half::f16;
use image::flat::View;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BlendComponent, BlendState, Buffer, BufferBinding, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, FragmentState, MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState, RenderPass, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStages, Surface, SurfaceError, Texture, TextureDimension, TextureUsages, TextureView, VertexState, util::{BufferInitDescriptor, DeviceExt}, wgt::{BufferDescriptor, TextureDescriptor, TextureViewDescriptor}};

use crate::{gputil::{GPUContext, extent_2d}, line::DisplayHandle, shaders, viewport::{ViewportUniforms, ViewportWindow}};

// GPU-friendly colour values. These are not gamma-encoded
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, align(8))]
pub struct RGBA16f {
    pub r: f16,
    pub g: f16,
    pub b: f16,
    pub a: f16,
}

impl RGBA16f {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        RGBA16f {
            r: f16::from_f32_const(r),
            g: f16::from_f32_const(g),
            b: f16::from_f32_const(b),
            a: f16::from_f32_const(a),
         }
    }

    pub fn to_wgpu_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r.into(),
            g: self.g.into(),
            b: self.b.into(),
            a: self.a.into(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, align(8))]
pub struct GridParams {
    pub line_spacing: f32,
    pub major_every: u32,
    pub line_color: RGBA16f,
    pub major_color: RGBA16f,
    pub axis_color: RGBA16f,
    pub background_color: RGBA16f,
}

fn num_grid_lines(view: &ViewportWindow, params: &GridParams) -> u32 {
    let (sw, ne) = view.as_rect();
    ((ne - sw) / params.line_spacing as f64).floor().as_uvec2().max_element()
}


pub struct LineEditRenderer {
    gpu: GPUContext,

    view_bg_layout: BindGroupLayout,
    current_view: ViewportWindow,
    view_unif: Buffer,
    view_bg: BindGroup,

    grid_pipeline: RenderPipeline,
    current_grid: GridParams,
    grid_unif: Buffer,
    grid_bg: BindGroup,

    handle_pipeline: RenderPipeline,
    num_handles: usize,
    handle_buf: Buffer,
    handle_bg: BindGroup,

    spline_pipeline: RenderPipeline,
    num_splines: usize,
    spline_buf: Buffer,
    spline_bg: BindGroup,

    spiral_test_pipeline: RenderPipeline,

    msaa_tex: Texture,
    msaa_view: TextureView,
    canvas: Surface<'static>,
    last_size: UVec2,
}

impl LineEditRenderer {
    const MAX_HANDLES: usize = 1024;
    const MAX_SPLINE_SEGS: usize = 128;

    pub fn new(
        gpu: &GPUContext,
        canvas: Surface<'static>,
        size: UVec2,
        view: &ViewportWindow,
        grid: &GridParams,
    ) -> Self {
        let view_bg_layout = gpu.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("grid_bg_layout"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: NonZero::new(size_of::<ViewportUniforms>() as u64)},
                    count: None,
                }
            ]
        });

        let grid_shaders = gpu.process_shader_module("grid.wgsl", shaders::GRID);
        let grid_bg_layout = gpu.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("grid_bg_layout"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: NonZero::new(size_of::<GridParams>() as u64)},
                    count: None,
                }
            ]
        });
        let grid_pipeline_layout = gpu.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("grid_pipeline_layout"),
            bind_group_layouts: &[&view_bg_layout, &grid_bg_layout],
            immediate_size: 0,
        });
        let grid_pipeline = gpu.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("grid_pipeline"),
            layout: Some(&grid_pipeline_layout),
            vertex: VertexState {
                module: &grid_shaders,
                entry_point: Some("grid_line_vert"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &grid_shaders,
                entry_point: Some("grid_line_frag"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState{
                    format: gpu.output_format,
                    blend: Some(BlendState {
                        color: BlendComponent::OVER,
                        alpha: BlendComponent::OVER,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let view_unif = gpu.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("view_unif"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&view.to_uniforms()),
        });

        let view_bg = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("view_bg"),
            layout: &view_bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &view_unif,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let grid_unif = gpu.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("grid_unif"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(grid),
        });

        let grid_bg = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("grid_bg"),
            layout: &grid_bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: grid_unif.as_entire_binding(),
            }],
        });

        let handle_shaders = gpu.process_shader_module("handles.wgsl", shaders::HANDLES);
        let handle_bg_layout = gpu.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("handle_bg_layout"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None},
                    count: None,
                }
            ]
        });
        let handle_pipeline_layout = gpu.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("handle_pipeline_layout"),
            bind_group_layouts: &[&view_bg_layout, &handle_bg_layout],
            immediate_size: 0,
        });
        let handle_pipeline = gpu.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("handle_pipeline"),
            layout: Some(&handle_pipeline_layout),
            vertex: VertexState {
                module: &handle_shaders,
                entry_point: Some("handles_vert"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &handle_shaders,
                entry_point: Some("handles_frag"),
                compilation_options: Default::default(),
                targets: &[Some(gpu.output_format.into())],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let handle_buf = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("handle_buf"),
            size: (Self::MAX_HANDLES * size_of::<DisplayHandle>()) as u64, 
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let handle_bg = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("handle_bg"),
            layout: &handle_bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: handle_buf.as_entire_binding(),
            }],
        });

        let spline_shaders = gpu.process_shader_module("spline_plot.wgsl", shaders::SPLINE_PLOT);
        let spline_bg_layout = gpu.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("spline_bg_layout"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None},
                    count: None,
                }
            ]
        });
        let spline_pipeline_layout = gpu.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("spline_pipeline_layout"),
            bind_group_layouts: &[&view_bg_layout, &spline_bg_layout],
            immediate_size: 0,
        });
        let spline_pipeline = gpu.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("spline_pipeline"),
            layout: Some(&spline_pipeline_layout),
            vertex: VertexState {
                module: &spline_shaders,
                entry_point: Some("spline_plot_vert"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &spline_shaders,
                entry_point: Some("spline_plot_frag"),
                compilation_options: Default::default(),
                targets: &[Some(gpu.output_format.into())],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let spline_buf = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("spline_buf"),
            size: (Self::MAX_SPLINE_SEGS * size_of::<ClothoidSegParams>()) as u64, 
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let spline_bg = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("spline_bg"),
            layout: &spline_bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: spline_buf.as_entire_binding(),
            }],
        });

        let spiral_shaders = gpu.process_shader_module("spiral_test.wgsl", shaders::SPIRAL_TEST);
        let spiral_pipeline_layout = gpu.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("spiral_pipeline_layout"),
            bind_group_layouts: &[&view_bg_layout],
            immediate_size: 0,
        });
        let spiral_test_pipeline = gpu.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("spiral_pipeline"),
            layout: Some(&spiral_pipeline_layout),
            vertex: VertexState {
                module: &spiral_shaders,
                entry_point: Some("spiral_test_vert"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &spiral_shaders,
                entry_point: Some("spiral_test_frag"),
                compilation_options: Default::default(),
                targets: &[Some(gpu.output_format.into())],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let msaa_tex = gpu.device.create_texture(&TextureDescriptor {
            label: Some("msaa_tex"),
            size: extent_2d(size),
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format: gpu.output_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TRANSIENT,
            view_formats: &[],
        });
        let msaa_view = msaa_tex.create_view(&Default::default());

        gpu.configure_surface_target(&canvas, size);

        LineEditRenderer {
            gpu: gpu.clone(),
            view_bg_layout,
            grid_pipeline,
            view_unif, view_bg, current_view: *view,
            grid_unif, grid_bg, current_grid: *grid,
            handle_pipeline,
            handle_buf, handle_bg, num_handles: 0,
            spline_pipeline,
            spline_buf, spline_bg, num_splines: 0,
            spiral_test_pipeline,
            msaa_tex, msaa_view, canvas,
            last_size: size
        }
    }

    pub fn resize(&mut self, size: UVec2) {
        if self.last_size == size { return; }

        self.msaa_tex.destroy();
        self.msaa_tex = self.gpu.device.create_texture(&TextureDescriptor {
            label: Some("msaa_tex"),
            size: extent_2d(size),
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format: self.gpu.output_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TRANSIENT,
            view_formats: &[],
        });
        self.msaa_view = self.msaa_tex.create_view(&Default::default());
        self.gpu.configure_surface_target(&self.canvas, size);
        log::info!("resize msaa buffer {}", size);

        self.last_size = size;
    }

    pub fn set_viewport(&mut self, view: &ViewportWindow) {
        self.current_view = *view;
        self.gpu.queue.write_buffer(&self.view_unif, 0, bytes_of(&view.to_uniforms()));
    }

    pub fn set_grid_params(&mut self, params: &GridParams) {
        self.gpu.queue.write_buffer(&self.grid_unif, 0, bytes_of(params));
    }

    pub fn set_handles(&mut self, handles: &[DisplayHandle]) {
        let mut num_handles = handles.len();
        if num_handles > Self::MAX_HANDLES {
            log::error!("too many handles to draw: {}", num_handles);
            num_handles = Self::MAX_HANDLES;
        }

        self.num_handles = num_handles;
        self.gpu.queue.write_buffer(&self.handle_buf, 0, bytemuck::cast_slice(&handles[0..num_handles]));
    }

    pub fn set_splines(&mut self, handles: &[ClothoidSegParams]) {
        let mut num_splines = handles.len();
        if num_splines > Self::MAX_SPLINE_SEGS {
            log::error!("too many spline segments to draw: {}", num_splines);
            num_splines = Self::MAX_SPLINE_SEGS;
        }

        self.num_splines = num_splines;
        self.gpu.queue.write_buffer(&self.spline_buf, 0, bytemuck::cast_slice(&handles[0..num_splines]));
    }

    pub fn render(&self) -> Result<(), SurfaceError> {
        let out_tex = self.canvas.get_current_texture()?;
        //log::info!("render");
        let out_view = out_tex.texture.create_view(&TextureViewDescriptor {
            format: Some(self.gpu.output_format),
            ..Default::default()
        });

        let mut command_encoder = self.gpu.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("gridded_renderer") });
        {
            let mut main_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.msaa_view,
                    depth_slice: None,
                    resolve_target: Some(&out_view),
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(self.current_grid.background_color.to_wgpu_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            main_pass.set_bind_group(0, &self.view_bg, &[]);

            let num_lines = num_grid_lines(&self.current_view, &self.current_grid);
            if num_lines != 0 {
                main_pass.set_pipeline(&self.grid_pipeline);
                main_pass.set_bind_group(1, &self.grid_bg, &[]);
                main_pass.draw(0..(2*num_lines), 0..2);
            }

            main_pass.set_pipeline(&self.spiral_test_pipeline);
            main_pass.draw(0..1000, 0..2);

            if self.num_handles != 0 {
                let verts = 6 * self.num_handles as u32;
                main_pass.set_pipeline(&self.handle_pipeline);
                main_pass.set_bind_group(1, &self.handle_bg, &[]);
                main_pass.draw(0..verts, 0..1);
            }

            if self.num_splines != 0 {
                let insts = self.num_splines as u32;
                main_pass.set_pipeline(&self.spline_pipeline);
                main_pass.set_bind_group(1, &self.spline_bg, &[]);
                main_pass.draw(0..101, 0..insts);
            }

        }

        self.gpu.queue.submit([command_encoder.finish()]);
        out_tex.present();
        Ok(())
    }
}

