use wgpu::*;
use wgpu::util::DeviceExt;
use glam::Vec2;
use crevice::std140::{AsStd140, Std140};

#[derive(AsStd140)]
pub struct BlurData {
    dir: Vec2,
    stdev: f32,
    radius: i32,
}

pub struct Blur {
    pub layout: BindGroupLayout,
    pub vertical_bind_group: BindGroup,
    pub horizontal_bind_group: BindGroup,
}

impl Blur {
    pub fn new(stdev: f32, radius: i32, device: &Device) -> Self {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("blur layout"),
        });

        let blur_data_horizontal = BlurData {
            dir: Vec2::new(1.0, 0.0),
            stdev,
            radius,
        };

        let blur_data_vertical = BlurData {
            dir: Vec2::new(0.0, 1.0),
            stdev,
            radius,
        };

        let buffer_horizontal = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("blur buffer horizontal"),
            contents: blur_data_horizontal.as_std140().as_bytes(),
            usage: BufferUsages::UNIFORM,
        });

        let buffer_vertical = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("blur buffer vertical"),
            contents: blur_data_vertical.as_std140().as_bytes(),
            usage: BufferUsages::UNIFORM,
        });

        let horizontal_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer_horizontal.as_entire_binding(),
                },
            ],
            label: Some("blur bind group horizontal"),
        });

        let vertical_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer_vertical.as_entire_binding(),
                },
            ],
            label: Some("blur bind group vertical"),
        });

        Self {
            layout,
            vertical_bind_group,
            horizontal_bind_group,
        }
    }
}
