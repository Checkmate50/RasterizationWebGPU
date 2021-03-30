use wgpu::*;
use anyhow::{Result, anyhow, Context as ErrorContext};
use winit::window::Window;
use shaderc::{Compiler, ShaderKind};
use crate::scene::Scene;
use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use std::path::Path;

fn make_shader_module(device: &Device, compiler: &mut Compiler, filename: impl AsRef<Path>, kind: ShaderKind) -> Result<ShaderModule> {
    let glsl_str = std::fs::read_to_string(filename)?;
    let spirv = compiler.compile_into_spirv(&glsl_str, kind, &format!("{:?}", kind), "main", None).context(format!("Error compiling {:?} shader", kind))?;
    let data = util::make_spirv(spirv.as_binary_u8());
    Ok(device.create_shader_module(&ShaderModuleDescriptor {
        label: None,
        source: data,
        flags: ShaderFlags::default(),
    }))
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    view_mat: Mat4,
    proj_mat: Mat4,
}

pub struct Context {
    device: Device,
    swap_chain: SwapChain,
    pipeline: RenderPipeline,
    depth_texture: Texture,
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
            let (device, queue) = adapter.request_device(&DeviceDescriptor::default(), None).await?;
            let format = adapter.get_swap_chain_preferred_format(&surface);

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


        // load and translate the shader modules into spirv
        let (vert_module, frag_module) = {
            let mut compiler = Compiler::new().ok_or(anyhow!("Couldn't initialize SPIR-V compiler"))?;

            let vert_module = make_shader_module(&device, &mut compiler, "resources/shaders/basic/shader.vert", ShaderKind::Vertex)?;

            let frag_module = make_shader_module(&device, &mut compiler, "resources/shaders/basic/shader.frag", ShaderKind::Fragment)?;

            (vert_module, frag_module)
        };

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

        // load mesh
        let scene = Scene::from_gltf(&device, &object_layout, &light_layout)?;

        // set up pipeline
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &scene.camera.layout,
                &object_layout,
                &light_layout,
            ],
            push_constant_ranges: &[],
            label: None,
        });

        let depth_texture = Texture::create_depth_texture(&device, width, height);

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            vertex: VertexState {
                module: &vert_module,
                entry_point: "main",
                buffers: &[
                    scene.meshes[0].get_vertex_desc(),
                ],
            },
            fragment: Some(FragmentState {
                module: &frag_module,
                entry_point: "main",
                targets: &[
                    ColorTargetState {
                        format,
                        alpha_blend: BlendState::default(),
                        color_blend: BlendState::default(),
                        write_mask: ColorWrite::default(),
                    }
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
            label: None,
        });

        Ok(Self {
            device,
            queue,
            swap_chain,
            pipeline,
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
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
                color_attachments: &[
                    RenderPassColorAttachmentDescriptor {
                        resolve_target: None,
                        attachment: &frame.view,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        }
                    }
                ],
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.scene.camera.bind_group, &[]);
            render_pass.set_bind_group(2, &self.scene.light_bind_group, &[]);
            for mesh in &self.scene.meshes {
                render_pass.set_bind_group(1, &mesh.bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                render_pass.set_index_buffer(mesh.indices.slice(..), IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.length, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }
}
