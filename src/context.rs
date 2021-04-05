use wgpu::*;
use anyhow::{Result, anyhow};
use winit::window::Window;
use crate::scene::Scene;
use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use std::borrow::Cow;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    view_mat: Mat4,
    proj_mat: Mat4,
}

pub struct Context {
    device: Device,
    swap_chain: SwapChain,
    deferred_pipeline: RenderPipeline,
    shading_pipeline: RenderPipeline,
    depth_texture: Texture,
    material_texture: Texture,
    diffuse_texture: Texture,
    normal_texture: Texture,
    pub scene: Scene,
    pub queue: Queue,
}

impl Context {
    pub async fn new(window: &Window) -> Result<Self> {

        let width = window.inner_size().width;
        let height = window.inner_size().height;

        // some initial state
        let (device, queue, swap_chain, format) = {

            // create device, queue
            let instance = Instance::new(BackendBit::PRIMARY);
            let surface = unsafe { instance.create_surface(window) };
            let adapter = instance.request_adapter(
                &RequestAdapterOptionsBase {
                    power_preference: PowerPreference::default(),
                    compatible_surface: Some(&surface),
                }
            ).await.ok_or(anyhow!("Couldn't get adapter"))?;
            let (device, queue) = adapter.request_device(
                &DeviceDescriptor{
                    limits: wgpu::Limits {
                        max_bind_groups: 6,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Some(&std::path::Path::new("path"))
                ).await?;
            let format = adapter.get_swap_chain_preferred_format(&surface).ok_or(anyhow!("Incompatible surface!"))?;

            // create swap chain
            let swap_chain = device.create_swap_chain(&surface, &SwapChainDescriptor {
                usage: TextureUsage::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode: PresentMode::Fifo,
            });

            (device, queue, swap_chain, format)
        };

        // create required layouts
        let (object_layout, light_layout, texture_layout, depth_layout) = {
            let object_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: None,
            });

            let light_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: None,
            });

            let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Sampler {
                            filtering: false,
                            comparison: false,
                        },
                        count: None,
                    }
                ],
                label: None,
            });

            let depth_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Sampler {
                            filtering: false,
                            comparison: false,
                        },
                        count: None,
                    }
                ],
                label: None,
            });
            (object_layout, light_layout, texture_layout, depth_layout)
        };

        // load mesh
        let scene = Scene::from_gltf(&device, &object_layout, &light_layout)?;

        // set up deferred pipeline
        let deferred_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &scene.camera.layout,
                    &object_layout,
                ],
                push_constant_ranges: &[],
                label: None,
            });

            let shader = {
                let shader_str = std::fs::read_to_string("resources/shaders/wgsl/deferred.wgsl")?;
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("deferred module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
                    flags: ShaderFlags::default(),
                })
            };

            device.create_render_pipeline(&RenderPipelineDescriptor {
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[
                        scene.meshes[0].get_vertex_desc(),
                    ],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[
                        ColorTargetState {
                            format: TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: ColorWrite::default(),
                        },
                        ColorTargetState {
                            format: TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: ColorWrite::default(),
                        },
                        ColorTargetState {
                            format: TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: ColorWrite::default(),
                        },
                    ],
                }),
                layout: Some(&layout),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Less,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                    clamp_depth: false,
                }),
                label: Some("deferred pipeline"),
            })
        };

        // set up shading pipeline
        let shading_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &scene.camera.layout,
                    &light_layout,
                    &texture_layout,
                    &texture_layout,
                    &texture_layout,
                    &depth_layout,
                ],
                push_constant_ranges: &[],
                label: None,
            });

            let shader = {
                let shader_str = std::fs::read_to_string("resources/shaders/wgsl/shading.wgsl")?;
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("shading module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
                    flags: ShaderFlags::VALIDATION,
                })
            };

            device.create_render_pipeline(&RenderPipelineDescriptor {
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[
                        ColorTargetState {
                            format,
                            blend: None,
                            write_mask: ColorWrite::default(),
                        },
                    ],
                }),
                layout: Some(&layout),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: None,
                label: Some("shading pipeline"),
            })
        };

        let diffuse_texture = Texture::create_window_texture(&device, &texture_layout, width, height);
        let material_texture = Texture::create_window_texture(&device, &texture_layout, width, height);
        let normal_texture = Texture::create_window_texture(&device, &texture_layout, width, height);
        let depth_texture = Texture::create_depth_texture(&device, &depth_layout, width, height);

        Ok(Self {
            device,
            queue,
            swap_chain,
            deferred_pipeline,
            shading_pipeline,
            material_texture,
            diffuse_texture,
            normal_texture,
            scene,
            depth_texture,
        })
    }

    pub fn render(&self) -> Result<()> {
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
                color_attachments: &[
                    RenderPassColorAttachment {
                        resolve_target: None,
                        view: &self.diffuse_texture.view,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        }
                    },
                    RenderPassColorAttachment {
                        resolve_target: None,
                        view: &self.material_texture.view,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        }
                    },
                    RenderPassColorAttachment {
                        resolve_target: None,
                        view: &self.normal_texture.view,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        }
                    },
                ],
            });

            render_pass.set_pipeline(&self.deferred_pipeline);
            render_pass.set_bind_group(0, &self.scene.camera.bind_group, &[]);
            for mesh in &self.scene.meshes {
                render_pass.set_bind_group(1, &mesh.bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                render_pass.set_index_buffer(mesh.indices.slice(..), IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.length, 0, 0..1);
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                depth_stencil_attachment: None,
                color_attachments: &[
                    RenderPassColorAttachment {
                        resolve_target: None,
                        view: &frame.view,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        }
                    }
                ],
            });

            render_pass.set_pipeline(&self.shading_pipeline);
            render_pass.set_bind_group(0, &self.scene.camera.bind_group, &[]);
            render_pass.set_bind_group(1, &self.scene.light_bind_group, &[]);
            render_pass.set_bind_group(2, &self.diffuse_texture.bind_group, &[]);
            render_pass.set_bind_group(3, &self.normal_texture.bind_group, &[]);
            render_pass.set_bind_group(4, &self.material_texture.bind_group, &[]);
            render_pass.set_bind_group(5, &self.depth_texture.bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }
}
