use wgpu::*;
use wgpu::util::DeviceExt;
use serde::{Serialize, Deserialize};
use glam::{Vec3, Vec2, Mat4};
use std::path::Path;
use anyhow::Result;
use crevice::std140::{AsStd140, Std140};
use mint::Vector3;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum LightJSON {
    Point {
        node: String,
        position: Vec3,
        power: Vec3,
    },
    Area {
        node: String,
        position: Vec3,
        power: Vec3,
        normal: Vec3,
        up: Vec3,
        size: Vec2,
        #[serde(default = "Vec3::default")]
        u: Vec3,
        #[serde(default = "Vec3::default")]
        v: Vec3,
    },
    Ambient {
        node: String,
        radiance: Vec3,
        range: Option<f32>,
    },
}

impl LightJSON {
    pub fn from_bytes(bytes: &[u8]) -> Result<Vec<Self>> {
        Ok(serde_json::from_slice(&bytes)?) // kinda weird
    }

    pub fn from_file(filename: impl AsRef<Path>) -> Result<Vec<Self>> {
        let json_str = std::fs::read_to_string(filename)?;
        Ok(serde_json::from_str(&json_str)?)
    }

    pub fn get_node(&self) -> &str {
        match self {
            LightJSON::Point { node, ..} | LightJSON::Area { node, .. } | LightJSON::Ambient { node, .. } => node
        }
    }

    pub fn apply_matrix(&mut self, mat: Mat4) {
        match self {
            LightJSON::Point { position, .. } => {
                *position = mat.transform_point3(*position);
            },
            LightJSON::Area { position, normal, up, u, v, .. } => {
                *position = mat.transform_point3(*position);
                *normal = mat.transform_vector3(*normal);
                *up = mat.transform_vector3(*up);

                // compute light basis here to not do later
                *u = up.cross(*normal).normalize();
                *v = normal.cross(*u);
            },
            LightJSON::Ambient { .. } => (),
        }
    }
}

#[derive(AsStd140)]
pub struct Light {
    power: Vector3<f32>,
    position: Vector3<f32>,
}

impl Light {
    pub fn new(position: Vec3, power: Vec3) -> Self {
        Self {
            position: position.into(),
            power: power.into(),
        }
    }

    pub fn to_bind_group(self, device: &Device, layout: &BindGroupLayout) -> BindGroup {
        let buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: self.as_std140().as_bytes(),
            usage: BufferUsage::UNIFORM,
        });

        device.create_bind_group(&BindGroupDescriptor {
            layout: layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }
            ],
            label: None,
        })
    }
}
