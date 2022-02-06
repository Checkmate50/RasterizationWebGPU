use wgpu::*;
use wgpu::util::DeviceExt;
use serde::{Serialize, Deserialize};
use glam::{Vec3, Vec2, Mat4};
use crate::texture::Texture;
use std::path::Path;
use anyhow::Result;

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

pub enum Light {
    Point { texture: Texture, slice: [auto] },
    Ambient { buffer: Buffer },
}

impl Light {
    pub fn new_point(position: Vec3, power: Vec3, device: &Device, _layout: &BindGroupLayout, texture_layout: &BindGroupLayout) -> Self {
        let view_mat = Mat4::look_at_rh(position, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let proj_mat = Mat4::perspective_rh(1.0, 1.0, 1.0, 50.0);

        let slice = [
            proj_mat.col(0),
            proj_mat.col(1),
            proj_mat.col(2),
            proj_mat.col(3),
            view_mat.col(0),
            view_mat.col(1),
            view_mat.col(2),
            view_mat.col(3),
            power.extend(1.0),
            position.extend(1.0),
        ];

        let texture = Texture::create_window_texture(&device, &texture_layout, TextureFormat::Depth32Float, Some(CompareFunction::LessEqual), 1024, 1024);
        Self::Point {
            texture,
            slice
        }
    }

    pub fn new_ambient(radiance: Vec3, range: Option<f32>, device: &Device, _layout: &BindGroupLayout) -> Self {

        let slice = [
            radiance.x,
            radiance.y,
            radiance.z,
            range.unwrap_or(0.0),
        ];

        let buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("ambient light buffer"),
            contents: bytemuck::cast_slice(&slice),
            usage: BufferUsages::UNIFORM,
        });

        Self::Ambient {
            buffer,
        }
    }
}
