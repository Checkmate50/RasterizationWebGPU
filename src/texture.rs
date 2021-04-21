use wgpu::*;

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

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                }
            ],
            label: Some(&format!("{:?}", format)),
        });


        Self { texture, view, sampler, bind_group, format }
    }
}

