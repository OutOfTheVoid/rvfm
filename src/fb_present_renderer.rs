use core::slice;
use shaderc;
use wgpu;
use crate::{fm_mio::FmMemoryIO};

pub struct FramebufferPresentRenderer {
	pipeline: wgpu::RenderPipeline,
	bind_group_layout: wgpu::BindGroupLayout,
}

impl FramebufferPresentRenderer {
	pub fn new(device: &wgpu::Device, swap_desc: &wgpu::SwapChainDescriptor) -> Result<Self, String> {
		let vs_src = include_str!("shaders/present.vert");
		let fs_src = include_str!("shaders/present.frag");
		let mut compiler = shaderc::Compiler::new().unwrap();
		let vs_spirv = compiler.compile_into_spirv(vs_src, shaderc::ShaderKind::Vertex, "mmfb_copy.vert", "main", None).unwrap();
		let fs_spirv = compiler.compile_into_spirv(fs_src, shaderc::ShaderKind::Fragment, "mmfb_copy.frag", "main", None).unwrap();
		let vs_module = device.create_shader_module(wgpu::util::make_spirv(&vs_spirv.as_binary_u8()));
		let fs_module = device.create_shader_module(wgpu::util::make_spirv(&fs_spirv.as_binary_u8()));
		let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStage::FRAGMENT,
					ty: wgpu::BindingType::SampledTexture {
						multisampled: false,
						dimension: wgpu::TextureViewDimension::D2,
						component_type: wgpu::TextureComponentType::Uint
					},
					count: None
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStage::FRAGMENT,
					ty: wgpu::BindingType::Sampler {
						comparison: false,
					},
					count: None,
				},
			],
			label: Some("copy bind group layout")
		});
		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("mmfb copy pipeline layout"),
			bind_group_layouts: &[&bind_group_layout],
			push_constant_ranges: &[],
		});
		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
    		layout: Some(&pipeline_layout),
			vertex_stage: wgpu::ProgrammableStageDescriptor {
				module: &vs_module,
				entry_point: "main",
			},
			fragment_stage: Some(wgpu::ProgrammableStageDescriptor { // 2.
				module: &fs_module,
				entry_point: "main",
			}),
			rasterization_state: Some(wgpu::RasterizationStateDescriptor {
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: wgpu::CullMode::None,
				depth_bias: 0,
				depth_bias_slope_scale: 0.0,
				depth_bias_clamp: 0.0,
				clamp_depth: false,
			}),
			color_states: &[
				wgpu::ColorStateDescriptor {
					format: swap_desc.format,
					color_blend: wgpu::BlendDescriptor::REPLACE,
					alpha_blend: wgpu::BlendDescriptor::REPLACE,
					write_mask: wgpu::ColorWrite::ALL,
				},
			],
			primitive_topology: wgpu::PrimitiveTopology::TriangleList,
			depth_stencil_state: None,
			vertex_state: wgpu::VertexStateDescriptor {
				index_format: wgpu::IndexFormat::Uint16,
				vertex_buffers: &[],
			},
			sample_count: 1,
			sample_mask: !0,
			alpha_to_coverage_enabled: false,
		});
		Ok(Self {
			pipeline,
			bind_group_layout,
		})
	}
	
	pub fn render(&mut self, device: &wgpu::Device, command_encoder: &mut wgpu::CommandEncoder, framebuffer: &wgpu::TextureView, present_buffer: &wgpu::Texture) {
		let copy_texture_view = present_buffer.create_view(&wgpu::TextureViewDescriptor::default());
		let copy_texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			..Default::default()
		});
		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &self.bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&copy_texture_view)
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(& copy_texture_sampler)
				}
			],
			label: Some("copy bind group")
		});
		{
			let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				color_attachments: &[
					wgpu::RenderPassColorAttachmentDescriptor {
						attachment: &framebuffer,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color {
								r: 1.0, g: 0.0, b: 0.0, a: 1.0
							}),
							store: true
						}
					}
				],
				depth_stencil_attachment: None,
			});
			render_pass.set_pipeline(&self.pipeline);
			render_pass.set_bind_group(0, &bind_group, &[]);
			render_pass.draw(0..6, 0..1);
		}
	}
}