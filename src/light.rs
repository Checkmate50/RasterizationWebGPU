use serde::{Serialize, Deserialize};
use glam::{Vec3, Vec2, Mat4};
use std::path::Path;
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Light {
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

impl Light {
    pub fn from_bytes(bytes: &[u8]) -> Result<Vec<Self>> {
        Ok(serde_json::from_slice(&bytes)?) // kinda weird
    }

    pub fn from_file(filename: impl AsRef<Path>) -> Result<Vec<Self>> {
        let json_str = std::fs::read_to_string(filename)?;
        Ok(serde_json::from_str(&json_str)?)
    }

    pub fn get_node(&self) -> &str {
        match self {
            Light::Point { node, ..} | Light::Area { node, .. } | Light::Ambient { node, .. } => node
        }
    }

    pub fn apply_matrix(&mut self, mat: Mat4) {
        match self {
            Light::Point { position, .. } => {
                *position = mat.transform_point3(*position);
            },
            Light::Area { position, normal, up, u, v, .. } => {
                *position = mat.transform_point3(*position);
                *normal = mat.transform_vector3(*normal);
                *up = mat.transform_vector3(*up);

                // compute light basis here to not do later
                *u = up.cross(*normal).normalize();
                *v = normal.cross(*u);
            },
            Light::Ambient { .. } => (),
        }
    }
}

