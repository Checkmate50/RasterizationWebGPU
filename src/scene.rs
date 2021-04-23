use wgpu::*;
use crate::mesh::Mesh;
use crate::camera::Camera;
use crate::material::Material;
use crate::sky::Sky;
use crate::light::{LightJSON, Light};
use anyhow::{Result, anyhow};
use glam::{Mat4, Vec3, Vec4};
use gltf::{Node, buffer::Data};
use std::f32::consts::PI;
use std::path::Path;

pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub sky: Sky,
}

impl Scene {
    pub fn from_gltf(device: &Device, mat_layout: &BindGroupLayout, light_layout: &BindGroupLayout, texture_layout: &BindGroupLayout, file_path: impl AsRef<Path>) -> Result<Self> {

        let json_path = file_path.as_ref().with_extension("json");
        let glb_path = file_path.as_ref().with_extension("glb");

        let (doc, buffers, _) = gltf::import(glb_path)?;
        let mut lights_raw = LightJSON::from_file(json_path)?;

        let materials = doc.materials().map(|m| {
            let a = m.pbr_metallic_roughness();
            Material::new(a.roughness_factor(), 1.0, 1.5, Vec4::from(a.base_color_factor()).into()).to_buffer(device)
        }).collect();

        // materials used in bunnyscene reference aren't actually ones in the gltf file, these are those
        //let materials = vec![
        //    Material::new(0.1, 1.0, 1.5, Vec3::new(0.2, 0.3, 0.8)).to_buffer(device),
        //    Material::new(0.2, 1.0, 1.5, Vec3::new(0.2, 0.2, 0.2)).to_buffer(device),
        //];

        let mut meshes = Vec::new();
        let mut maybe_camera = None;

        for node in doc.default_scene().unwrap().nodes() {
            parse_node(node, Mat4::IDENTITY, &mut meshes, &buffers, device, mat_layout, &mut maybe_camera, &mut lights_raw, &materials)?;
        }

        let camera = maybe_camera.unwrap_or(Camera::new(&device, Vec3::new(6.0, 8.0, 10.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.1, 50.0, 1.333, 0.26));

        let lights = lights_raw.into_iter().filter_map(|light| {
            match light {
                LightJSON::Point { position, power, .. } | LightJSON::Area { position, power, .. } => {
                    Some(Light::new_point(position, power, device, light_layout, texture_layout))
                },
                LightJSON::Ambient { radiance, range, .. } => {
                    Some(Light::new_ambient(radiance, range, device, light_layout))
                },
            }
        }).collect();

        let sky = Sky::new(80.0 * PI / 180.0, 8.0, device);

        Ok(Self {
            meshes,
            camera,
            lights,
            sky,
        })
    }

}

fn parse_node(node: Node, mut parent_mat: Mat4, meshes: &mut Vec<Mesh>, buffers: &Vec<Data>, device: &Device, layout: &BindGroupLayout, camera: &mut Option<Camera>, lights: &mut Vec<LightJSON>, materials: &Vec<Buffer>) -> Result<()> {
    parent_mat = parent_mat * Mat4::from_cols_array_2d(&node.transform().matrix());

    if let Some(name) = node.name() {
        if let Some(i) = lights.iter().position(|l| l.get_node() == name) {
            lights[i].apply_matrix(parent_mat);
        }
    }
    
    if camera.is_none() {
        *camera = node.camera().map(|c| Camera::from_gltf(device, c.projection(), parent_mat));
    }
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let mat_index = primitive.material().index().ok_or(anyhow!("Uh-oh, material without index. Shouldn't happen."))?;
            meshes.push(Mesh::from_gltf(device, &primitive, buffers, parent_mat, layout, &materials[mat_index])?);
        }
    }
    for node in node.children() {
        parse_node(node, parent_mat, meshes, buffers, device, layout, camera, lights, materials)?
    }

    Ok(())
}

