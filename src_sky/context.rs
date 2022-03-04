use wgpu::*;
use anyhow::{Result, anyhow};
use winit::window::Window;
use crate::scene::Scene;
use crate::light::Light;
use crate::blur::Blur;
use crate::texture::{Texture, MipTexture};
use std::borrow::Cow;
use include_wgsl::include_wgsl;
use std::path::Path;

pub struct Context {
    device: Device,
    surface: Surface,
    geometry_pipeline: RenderPipeline,
    shading_pipeline: RenderPipeline,
    post_pipeline: RenderPipeline,
    shadow_pipeline: RenderPipeline,
    ambient_pipeline: RenderPipeline,
    blur_pipeline: RenderPipeline,
    blit_pipeline: RenderPipeline,
    depth_texture: Texture,
    material_texture: Texture,
    diffuse_texture: Texture,
    normal_texture: Texture,
    blurred_texture_vertical: MipTexture,
    blurred_texture_horizontal: MipTexture,
    blurred_texture_all: MipTexture,
    blurs: [Blur; 4],
    pub scene: Scene,
    pub queue: Queue,
}

impl Context {
    pub async fn new(window: &Window, file_path: impl AsRef<Path>) -> Result<Self> {

        let width = window.inner_size().width;
        let height = window.inner_size().height;

        // some initial state
        let (device, surface, format, queue) = {

            // create device, queue
            let instance = Instance::new(Backends::PRIMARY);
            let surface = unsafe { instance.create_surface(window) };
            let adapter = instance.request_adapter(
                &RequestAdapterOptionsBase {
                    power_preference: PowerPreference::default(),
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }
            ).await.ok_or(anyhow!("Couldn't get adapter"))?;
            let (device, queue) = adapter.request_device(
                &DeviceDescriptor{
                    limits: wgpu::Limits {
                        max_bind_groups: 8, // set max number of bind groups to 8 as it defaults to 4
                        ..Default::default()
                    },
                    ..Default::default()
                },
                None,
                ).await?;
            let format = surface.get_preferred_format(&adapter).ok_or(anyhow!("Incompatible surface!"))?;

            surface.configure(&device, &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode: PresentMode::Immediate,
            });

            (device, surface, format, queue)
        };

        // create required layouts
        let (object_layout, light_layout, texture_layout, depth_layout, depth_layout_comparison) = {
            let object_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("object layout"),
            });

            let light_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("light layout"),
            });

            let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    }
                ],
                label: Some("texture layout"),
            });

            let depth_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    }
                ],
                label: Some("depth layout"),
            });

            let depth_layout_comparison = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler {
                            filtering: true,
                            comparison: true,
                        },
                        count: None,
                    },
                ],
                label: Some("depth layout comparison"),
            });
            (object_layout, light_layout, texture_layout, depth_layout, depth_layout_comparison)
        };

        // load mesh
        let scene = Scene::from_gltf(&device, &object_layout, &light_layout, &depth_layout_comparison, file_path)?;

        let blend_component = BlendComponent {
            operation: BlendOperation::Add,
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
        };

        // create required textures
        let diffuse_texture = Texture::create_window_texture(&device, &texture_layout, TextureFormat::Rgb10a2Unorm, None, width, height);
        let material_texture = Texture::create_window_texture(&device, &texture_layout, TextureFormat::Rgba16Float, None, width, height);
        let normal_texture = Texture::create_window_texture(&device, &texture_layout, TextureFormat::Rgba16Float, None, width, height);
        let depth_texture = Texture::create_window_texture(&device, &depth_layout, TextureFormat::Depth32Float, None, width, height);

        // set up geometry pipeline
        let geometry_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &scene.camera.layout,
                    &object_layout,
                ],
                push_constant_ranges: &[],
                label: Some("geometry pipeline layout"),
            });

            let shader = {
                let shader_str = include_wgsl!("./shaders/geometry.wgsl");
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("geometry module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
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
                        diffuse_texture.format.into(),
                        material_texture.format.into(),
                        normal_texture.format.into(),
                    ],
                }),
                layout: Some(&layout),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: depth_texture.format,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Less,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                label: Some("geometry pipeline"),
            })
        };

        // set up shadow pipeline
        let shadow_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &light_layout,
                    &object_layout,
                ],
                push_constant_ranges: &[],
                label: Some("shadow pipeline layout"),
            });

            let shader = {
                let shader_str = include_wgsl!("./shaders/shadow.wgsl");
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("shadow module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
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
                fragment: None,
                layout: Some(&layout),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState {
                        constant: 2,
                        slope_scale: 4.0,
                        clamp: 0.0,
                    },
                }),
                label: Some("shadow pipeline"),
            })
        };

        // pre-post blurred screen texture
        let num_mips = 5;
        let blurred_texture_vertical = MipTexture::new(&device, &texture_layout, width, height, num_mips);
        let blurred_texture_horizontal = MipTexture::new(&device, &texture_layout, width, height, num_mips);
        let blurred_texture_all = MipTexture::new(&device, &texture_layout, width, height, num_mips);

        // set up ambient pipeline
        let ambient_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &light_layout,
                    &scene.camera.layout,
                    &texture_layout,
                    &texture_layout,
                    &depth_layout,
                    &scene.sky.layout,
                ],
                push_constant_ranges: &[],
                label: Some("ambient pipeline"),
            });

            let shader = {
                let shader_str = include_wgsl!("./shaders/ambient.wgsl");
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("ambient module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
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
                            format: blurred_texture_all.format,
                            blend: Some(BlendState {
                                color: blend_component.clone(),
                                alpha: blend_component.clone(),
                            }),
                            write_mask: ColorWrites::default(),
                        },
                    ],
                }),
                layout: Some(&layout),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: None,
                label: Some("ambient pipeline"),
            })
        };

        // set up light pipeline
        let shading_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &light_layout,
                    &scene.camera.layout,
                    &texture_layout,
                    &texture_layout,
                    &depth_layout,
                    &texture_layout,
                    &depth_layout_comparison,
                ],
                push_constant_ranges: &[],
                label: Some("shading pipeline layout"),
            });

            let shader = {
                let shader_str = include_wgsl!("./shaders/shading.wgsl");
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("shading module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
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
                            format: blurred_texture_all.format,
                            blend: Some(BlendState {
                                color: blend_component.clone(),
                                alpha: blend_component,
                            }),
                            write_mask: ColorWrites::default(),
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


        let blurs = [
            Blur::new(3.1, 9, &device),
            Blur::new(6.225, 18, &device), 
            Blur::new(10.125, 30, &device), 
            Blur::new(16.4375, 48, &device), 
        ];

        // set up blur pipeline
        let blur_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &texture_layout,
                    &blurs[0].layout,
                ],
                push_constant_ranges: &[],
                label: Some("blur pipeline layout"),
            });

            let shader = {
                let shader_str = include_wgsl!("./shaders/blur.wgsl");
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("blur module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
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
                        blurred_texture_vertical.format.into(),
                    ],
                }),
                layout: Some(&layout),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: None,
                label: Some("blur pipeline"),
            })
        };

        // set up postprocess pipeline
        let post_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &texture_layout,
                    &texture_layout,
                    &texture_layout,
                    &texture_layout,
                    &texture_layout,
                ],
                push_constant_ranges: &[],
                label: Some("post pipeline layout"),
            });

            let shader = {
                let shader_str = include_wgsl!("./shaders/post.wgsl");
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("post module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
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
                        format.into(),
                    ],
                }),
                layout: Some(&layout),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: None,
                label: Some("post pipeline"),
            })
        };

        // set up blit pipeline
        let blit_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &texture_layout,
                ],
                push_constant_ranges: &[],
                label: Some("blit pipeline layout"),
            });

            let shader = {
                let shader_str = include_wgsl!("./shaders/blit.wgsl");
                device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("post module"),
                    source: ShaderSource::Wgsl(Cow::Borrowed(&shader_str)),
                })
            };

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blit pipeline"),
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[blurred_texture_vertical.format.into()],
                }),
                primitive: PrimitiveState::default(),
                multisample: MultisampleState::default(),
                depth_stencil: None,
            })
        };

        Ok(Self {
            device,
            surface,
            queue,
            geometry_pipeline,
            shading_pipeline,
            shadow_pipeline,
            blur_pipeline,
            post_pipeline,
            ambient_pipeline,
            material_texture,
            diffuse_texture,
            normal_texture,
            blurred_texture_vertical,
            blurred_texture_horizontal,
            blurred_texture_all,
            blit_pipeline,
            blurs,
            scene,
            depth_texture,
        })
    }

    pub fn render(&self, elapsed_time: f32) -> Result<()> {
        self.scene.animate(elapsed_time, &self.queue);
        let frame = self.surface.get_current_texture()?;
        let window_view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());

        let sky_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.scene.sky.layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.scene.sky.buffer.as_entire_binding(),
                },
            ],
            label: Some("sky bind group"),
        });

        // geometry pass
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("geometry pass"),
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

            render_pass.set_pipeline(&self.geometry_pipeline);
            render_pass.set_bind_group(0, &self.scene.camera.bind_group, &[]);
            for mesh in &self.scene.meshes {
                render_pass.set_bind_group(1, mesh.bind_group.as_ref().expect("Unbound mesh!"), &[]);
                render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                render_pass.set_index_buffer(mesh.indices.slice(..), IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.length, 0, 0..1);
            }
        }

        // shadow passes
        for light in &self.scene.lights {
            match light {
                Light::Point { texture, bind_group } => {
                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("shadow pass"),
                        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                            view: &texture.view,
                            depth_ops: Some(Operations {
                                load: LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        }),
                        color_attachments: &[],
                    });

                    render_pass.set_pipeline(&self.shadow_pipeline);
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    for mesh in &self.scene.meshes {
                        render_pass.set_bind_group(1, &mesh.bind_group.as_ref().expect("Unbound mesh!"), &[]);
                        render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                        render_pass.set_index_buffer(mesh.indices.slice(..), IndexFormat::Uint32);
                        render_pass.draw_indexed(0..mesh.length, 0, 0..1);
                    }
                },
                Light::Ambient { .. } => {},
            }
        }

        // shading pass
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shading pass"),
                depth_stencil_attachment: None,
                color_attachments: &[
                    RenderPassColorAttachment {
                        resolve_target: None,
                        view: &self.blurred_texture_vertical.views[0],
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        }
                    }
                ],
            });

            render_pass.set_pipeline(&self.shading_pipeline);
            render_pass.set_bind_group(1, &self.scene.camera.bind_group, &[]);
            render_pass.set_bind_group(2, &self.diffuse_texture.bind_group, &[]);
            render_pass.set_bind_group(3, &self.normal_texture.bind_group, &[]);
            render_pass.set_bind_group(4, &self.depth_texture.bind_group, &[]);
            render_pass.set_bind_group(5, &self.material_texture.bind_group, &[]);
            for light in &self.scene.lights {
                match light {
                    Light::Point { texture, bind_group } => {
                        render_pass.set_bind_group(6, &texture.bind_group, &[]);
                        render_pass.set_bind_group(0, &bind_group, &[]);
                        render_pass.draw(0..3, 0..1);
                    },
                    Light::Ambient { .. } => {},
                }
            }
            render_pass.set_pipeline(&self.ambient_pipeline);
            render_pass.set_bind_group(5, &sky_bind_group, &[]);
            for light in &self.scene.lights {
                match light {
                    Light::Ambient { bind_group } => {
                        render_pass.set_bind_group(0, &bind_group, &[]);
                        render_pass.draw(0..3, 0..1);
                    },
                    Light::Point { .. } => {},
                }
            }
        }

        self.blurred_texture_vertical.generate_mipmaps(&self.blit_pipeline, &mut encoder);

        // blur pass
        for i in 1..=4 {
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("blur pass 1"),
                    depth_stencil_attachment: None,
                    color_attachments: &[
                        RenderPassColorAttachment {
                            resolve_target: None,
                            view: &self.blurred_texture_horizontal.views[i],
                            ops: Operations {
                                load: LoadOp::Clear(Color::BLACK),
                                store: true,
                            }
                        }
                    ],
                });

                render_pass.set_pipeline(&self.blur_pipeline);
                render_pass.set_bind_group(0, &self.blurred_texture_vertical.bind_groups[i], &[]);
                render_pass.set_bind_group(1, &self.blurs[i - 1].vertical_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }

            // second blur pass
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("blur pass 2"),
                    depth_stencil_attachment: None,
                    color_attachments: &[
                        RenderPassColorAttachment {
                            resolve_target: None,
                            view: &self.blurred_texture_all.views[i],
                            ops: Operations {
                                load: LoadOp::Clear(Color::BLACK),
                                store: true,
                            }
                        }
                    ],
                });

                render_pass.set_pipeline(&self.blur_pipeline);
                render_pass.set_bind_group(0, &self.blurred_texture_horizontal.bind_groups[i], &[]);
                render_pass.set_bind_group(1, &self.blurs[i - 1].horizontal_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }

        // post pass
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("post pass"),
                depth_stencil_attachment: None,
                color_attachments: &[
                    RenderPassColorAttachment {
                        resolve_target: None,
                        view: &window_view,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: true,
                        }
                    }
                ],
            });

            render_pass.set_pipeline(&self.post_pipeline);
            render_pass.set_bind_group(0, &self.blurred_texture_vertical.bind_groups[0], &[]);
            for i in 1..5 {
                render_pass.set_bind_group(i, &self.blurred_texture_all.bind_groups[i as usize], &[]);
            }
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}
