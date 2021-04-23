use wgpu::*;
use wgpu::util::DeviceExt;
use core::num::NonZeroU32;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub bind_group: BindGroup,
    pub format: TextureFormat,
}

impl Texture {
    pub fn create_window_texture(device: &Device, layout: &BindGroupLayout, format: TextureFormat, compare: Option<CompareFunction>, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
            label: None,
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare,
            ..Default::default()
        });

        let dim_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[width as f32, height as f32]),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: dim_buffer.as_entire_binding(),
                }
            ],
            label: Some(&format!("{:?}", format)),
        });


        Self { texture, view, sampler, bind_group, format }
    }
}

pub struct MipTexture {
    pub texture: wgpu::Texture,
    pub sampler: Sampler,
    pub bind_groups: Vec<BindGroup>,
    pub views: Vec<TextureView>,
    pub format: TextureFormat,
    pub mip_level_count: u32,
}

impl MipTexture {
    pub fn new(device: &Device, layout: &BindGroupLayout, width: u32, height: u32, mip_level_count: u32) -> Self {
        let format = TextureFormat::Rgba32Float;

        let texture = device.create_texture(&TextureDescriptor {
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count,
            format,
            sample_count: 1,
            dimension: TextureDimension::D2,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
            label: None,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        let views: Vec<TextureView> = (0..mip_level_count).map(|i| {
            texture.create_view(&TextureViewDescriptor {
                label: Some("mip"),
                format: None,
                dimension: None,
                aspect: TextureAspect::All,
                base_mip_level: i,
                mip_level_count: NonZeroU32::new(1),
                base_array_layer: 0,
                array_layer_count: None,
            })
        }).collect();


        let bind_groups = (0..mip_level_count).map(|i| {
            let dim_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[(width >> i) as f32, (height >> i) as f32]),
                usage: BufferUsage::UNIFORM,
            });
            device.create_bind_group(&BindGroupDescriptor {
                layout: layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&views[i as usize]),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: dim_buffer.as_entire_binding(),
                    }
                ],
                label: Some(&format!("{:?} mip", format)),
            })
        }).collect();


        Self { texture, sampler, bind_groups, format, mip_level_count, views }
    }

    pub fn generate_mipmaps(&self, pipeline: &RenderPipeline, encoder: &mut CommandEncoder) {
        for i in 1..self.mip_level_count as usize {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[RenderPassColorAttachment {
                    view: &self.views[i],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::WHITE),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &self.bind_groups[i - 1], &[]);
            rpass.draw(0..4, 0..1);
        }
    }
}

