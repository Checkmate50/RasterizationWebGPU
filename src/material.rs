use wgpu::*;
use wgpu::util::DeviceExt;
use mint::Vector3;
use glam::Vec3;
use crevice::std140::{AsStd140, Std140};

#[derive(AsStd140)]
pub struct Material {
    alpha: f32,
    k_s: f32,
    eta: f32,
    diffuse: Vector3<f32>,
}

impl Material {
    pub fn new(alpha: f32, k_s: f32, eta: f32, diffuse: Vec3) -> Self {
        Self {
            alpha,
            k_s,
            eta,
            diffuse: diffuse.into(),
        }
    }

    pub fn to_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: self.as_std140().as_bytes(),
            usage: BufferUsage::UNIFORM,
        })
    }
}

