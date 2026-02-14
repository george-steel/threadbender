use core::num;

use bytemuck::bytes_of;
use glam::{Affine2, UVec2, Vec2};
use half::f16;
use image::flat::View;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BlendComponent, BlendState, Buffer, BufferBinding, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, FragmentState, MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState, RenderPass, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStages, Surface, SurfaceError, Texture, TextureDimension, TextureUsages, TextureView, VertexState, wgt::{BufferDescriptor, TextureDescriptor, TextureViewDescriptor}};

use crate::{gputil::{GPUContext, extent_2d}, shaders, viewport::{ViewportUniforms, ViewportWindow}};

// GPU-friendly colour values. These are not gamma-encoded
#[derive(Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, align(8))]
pub struct RGBA16f {
    pub r: f16,
    pub g: f16,
    pub b: f16,
    pub a: f16,
}

impl RGBA16f {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        RGBA16f {
            r: f16::from_f32(r),
            g: f16::from_f32(g),
            b: f16::from_f32(b),
            a: f16::from_f32(a),
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

#[derive(Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GridUniforms {
    pub viewport: ViewportUniforms,
    pub params: GridParams,
}

impl GridUniforms {
    pub fn num_lines(&self) -> UVec2 {
        ((self.viewport.ne - self.viewport.sw) / self.params.line_spacing).floor().as_uvec2()
    }
}

pub struct GriddedRenderer {
    grid_shaders: ShaderModule,
    grid_pipeline: RenderPipeline,
    grid_unif: Buffer,
    grid_bg: BindGroup,
    msaa_tex: Texture,
    msaa_view: TextureView,
    canvas: Surface<'static>,
    last_size: UVec2,
}

impl GriddedRenderer {
    pub fn new(gpu: &GPUContext, canvas: Surface<'static>, size: UVec2) -> Self {
        let grid_shaders = gpu.process_shader_module("grid.wgsl", shaders::GRID);
        let grid_bg_layout = gpu.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("grid_bg_layout"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                }
            ]
        });
        let grid_pipeline_layout = gpu.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("grid_pipeline_layout"),
            bind_group_layouts: &[&grid_bg_layout],
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

        let grid_unif = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("grid_unif"),
            size: size_of::<GridUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_bg = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("grid_bg"),
            layout: &grid_bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &grid_unif,
                    offset: 0,
                    size: None,
                }),
            }],
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

        GriddedRenderer {
            grid_shaders, grid_pipeline,
            grid_unif, grid_bg,
            msaa_tex, msaa_view, canvas,
            last_size: size
        }
    }

    pub fn resize(&mut self, gpu: &GPUContext, size: UVec2) {
        if self.last_size == size { return; }

        self.msaa_tex.destroy();
        self.msaa_tex = gpu.device.create_texture(&TextureDescriptor {
            label: Some("msaa_tex"),
            size: extent_2d(size),
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format: gpu.output_format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TRANSIENT,
            view_formats: &[],
        });
        self.msaa_view = self.msaa_tex.create_view(&Default::default());
        gpu.configure_surface_target(&self.canvas, size);
        log::info!("resize msaa buffer {}", size);

        self.last_size = size;
    }

    pub fn render(&self,
        gpu: &GPUContext,
        grid_uniforms: &GridUniforms,
        mut render_children: impl FnMut(&GPUContext, &mut RenderPass),
    ) -> Result<(), SurfaceError> {
        let out_tex = self.canvas.get_current_texture()?;
        let out_view = out_tex.texture.create_view(&TextureViewDescriptor {
            format: Some(gpu.output_format),
            ..Default::default()
        });

        gpu.queue.write_buffer(&self.grid_unif, 0, bytes_of(grid_uniforms));

        let mut command_encoder = gpu.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("gridded_renderer") });

        {
            let mut main_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.msaa_view,
                    depth_slice: None,
                    resolve_target: Some(&out_view),
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: grid_uniforms.params.background_color.r.to_f64(),
                            g: grid_uniforms.params.background_color.g.to_f64(),
                            b: grid_uniforms.params.background_color.b.to_f64(),
                            a: grid_uniforms.params.background_color.a.to_f64(),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            main_pass.set_pipeline(&self.grid_pipeline);
            main_pass.set_bind_group(0, &self.grid_bg, &[]);
            
            let num_lines = grid_uniforms.num_lines().max_element();
            log::info!("drawing {} lines", num_lines);
            if num_lines != 0 {
                main_pass.draw(0..(2*num_lines), 0..2);
            }

            render_children(&gpu, &mut main_pass);
        }

        gpu.queue.submit([command_encoder.finish()]);
        out_tex.present();
        Ok(())
    }
}

