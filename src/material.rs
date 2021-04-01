use wgpu::*;
use wgpu::util::DeviceExt;
use glam::Vec3;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Material {
    alpha: f32,
    k_s: f32,
    eta: f32,
    diffuse: Vec3,
}

impl Material {
    pub fn new(alpha: f32, k_s: f32, eta: f32, diffuse: Vec3) -> Self {
        Self {
            alpha,
            k_s,
            eta,
            diffuse,
        }
    }

    pub fn to_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(self),
            usage: BufferUsage::UNIFORM,
        })
    }
}

