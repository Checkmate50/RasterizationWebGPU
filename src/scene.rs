use wgpu::*;
use crate::mesh::Mesh;
use crate::camera::Camera;
use crate::material::Material;
use crate::sky::Sky;
use crate::light::{LightJSON, Light};
use crate::animation::{Animation, Transformation};
use anyhow::Result;
use glam::{Mat4, Vec3, Vec4};
use gltf::{Node, buffer::Data, Document};
use std::f32::consts::PI;
use std::path::Path;

pub struct Scene {
    pub camera: Camera,
    pub sky: Sky,
    pub meshes: Vec<Mesh>,
    pub lights: Vec<Light>,
    pub skins: Vec<Vec<(usize, Mat4)>>,
    pub animations: Vec<Animation>,
    pub source: Document,
}

impl Scene {
    pub fn from_gltf(device: &Device, mat_layout: &BindGroupLayout, light_layout: &BindGroupLayout, texture_layout: &BindGroupLayout, file_path: impl AsRef<Path>) -> Result<Self> {

        let json_path = file_path.as_ref().with_extension("json");
        let glb_path = file_path.as_ref().with_extension("glb");

        let (source, buffers, _) = gltf::import(glb_path)?;
        let mut lights_raw = LightJSON::from_file(json_path)?;

        let materials = source.materials().map(|m| {
            let a = m.pbr_metallic_roughness();
            Material::new(a.roughness_factor(), 1.0, 1.5, Vec4::from(a.base_color_factor()).into()).to_buffer(device)
        }).collect();

        let animations = source.animations().map(|a| {
            let (min, max) = a.samplers().map(|a| {
                let min = a.input().min().unwrap().as_array().unwrap()[0].as_f64().unwrap();
                let max = a.input().max().unwrap().as_array().unwrap()[0].as_f64().unwrap();
                (min, max)
            }).fold((0.0_f64, 0.0_f64), |acc, x| (acc.0.min(x.0), acc.1.max(x.1)));
            let duration = (max - min) as f32;
            let ref_buffers = &buffers;
            a.channels().map(move |c| Animation::new(c, ref_buffers, duration))
        }).flatten().collect();

        // materials used in bunnyscene reference aren't actually ones in the gltf file, these are those
        //let materials = vec![
        //    Material::new(0.1, 1.0, 1.5, Vec3::new(0.2, 0.3, 0.8)).to_buffer(device),
        //    Material::new(0.2, 1.0, 1.5, Vec3::new(0.2, 0.2, 0.2)).to_buffer(device),
        //];

        let skins = source.skins().map(|skin| {
            skin.reader(|buffer| Some(&buffers[buffer.index()]))
                .read_inverse_bind_matrices()
                .into_iter()
                .flatten()
                .map(|a| Mat4::from_cols_array_2d(&a))
                .chain(std::iter::repeat(Mat4::IDENTITY))
                .zip(skin.joints())
                .map(|(j, n)| (n.index(), j))
                .collect::<Vec<(usize, Mat4)>>()
        }).collect::<Vec<Vec<(usize, Mat4)>>>();

        let mut transforms = skins.iter().map(|v| v.iter().map(|i| (i.0, Mat4::IDENTITY)).collect::<Vec<(usize, Mat4)>>()).collect::<Vec<Vec<(usize, Mat4)>>>();

        let mut meshes = Vec::new();
        let mut maybe_camera = None;

        for node in source.default_scene().unwrap().nodes() {
            parse_node(node, Mat4::IDENTITY, &mut meshes, &buffers, device, mat_layout, &mut maybe_camera, &mut lights_raw, &materials, &mut transforms)?;
        }

        let camera = maybe_camera.unwrap_or(Camera::new(&device, Vec3::new(6.0, 8.0, 10.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.1, 50.0, 1.333, 0.5));

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

        for mesh in meshes.iter_mut() {
            let joint_matrices = if let Some(i) = mesh.skin_index {
                let inv_mesh_mat = (*mesh.matrix.borrow()).inverse();
                transforms[i].iter().zip(skins[i].iter()).map(|(t, j)| inv_mesh_mat * t.1 * j.1).collect::<Vec<Mat4>>()
            } else {
                vec![Mat4::IDENTITY]
            };
            if let Some(index) = mesh.mat_index {
                mesh.bind(device, mat_layout, &joint_matrices, &materials[index])
            } else {
                let material = Material::new(0.5, 1.0, 1.5, Vec3::new(0.5, 0.5, 0.5)).to_buffer(device);
                mesh.bind(device, mat_layout, &joint_matrices, &material)
            }
        }

        Ok(Self {
            meshes,
            camera,
            lights,
            sky,
            animations,
            source,
            skins,
        })
    }

    pub fn animate(&self, time: f32, queue: &Queue) {
        let mut transforms = self.skins.iter().map(|v| v.iter().map(|i| (i.0, Mat4::IDENTITY)).collect::<Vec<(usize, Mat4)>>()).collect::<Vec<Vec<(usize, Mat4)>>>();
        for node in self.source.default_scene().unwrap().nodes() {
            self.animate_node(node, Mat4::IDENTITY, time, queue, &mut transforms);
        }
        for mesh in &self.meshes {
            let joint_matrices = if let Some(i) = mesh.skin_index {
                let inv_mesh_mat = (*mesh.matrix.borrow()).inverse();
                transforms[i].iter().zip(self.skins[i].iter()).map(|(t, j)| inv_mesh_mat * t.1 * j.1).collect::<Vec<Mat4>>()
            } else {
                vec![Mat4::IDENTITY]
            };
            mesh.update_joints(queue, &joint_matrices);
        }
    }

    fn animate_node(&self, node: Node, mut parent_mat: Mat4, time: f32, queue: &Queue, transforms: &mut Vec<Vec<(usize, Mat4)>>) {
        let mut rotation = Mat4::IDENTITY;
        let mut translation = Mat4::IDENTITY;
        let mut scale = Mat4::IDENTITY;
        let mut animated = false;
        for animation in &self.animations {
            if animation.target == node.index() {
                if let Some(transform) = animation.get(time) {
                    animated = true;
                    match transform {
                        Transformation::Translate(v) => translation = translation * Mat4::from_translation(v),
                        Transformation::Rotate(q) => rotation = rotation * Mat4::from_quat(q),
                        Transformation::Scale(s) => scale = scale * Mat4::from_scale(s),
                    }
                }
            }
        }
        if animated {
            parent_mat = parent_mat * translation * rotation * scale;
        } else {
            parent_mat = parent_mat * Mat4::from_cols_array_2d(&node.transform().matrix());
        }
        if let Some(mesh) = node.mesh() {
            self.meshes.iter()
                .find(|m| m.index == mesh.index())
                .unwrap()
                .update_transforms(queue, parent_mat);
        }
        for transform in transforms.iter_mut() {
            for i in transform {
                if i.0 == node.index() {
                    i.1 = parent_mat;
                }
            }
        }
        for node in node.children() {
            self.animate_node(node, parent_mat, time, queue, transforms);
        }
    }
}

// after looking at some other gltf viewer implementations, I realize this is an absolutely
// terrible way to do this, and I would greatly benefit from implementing this in a more
// flexible/easier way.
// However: sunk cost fallacy
fn parse_node(node: Node, mut parent_mat: Mat4, meshes: &mut Vec<Mesh>, buffers: &Vec<Data>, device: &Device, layout: &BindGroupLayout, camera: &mut Option<Camera>, lights: &mut Vec<LightJSON>, materials: &Vec<Buffer>, transforms: &mut Vec<Vec<(usize, Mat4)>>) -> Result<()> {
    parent_mat = parent_mat * Mat4::from_cols_array_2d(&node.transform().matrix());

    if let Some(name) = node.name() {
        if let Some(i) = lights.iter().position(|l| l.get_node() == name) {
            lights[i].apply_matrix(parent_mat);
        }
    }
    
    if camera.is_none() {
        *camera = node.camera().map(|c| Camera::from_gltf(device, c.projection(), parent_mat));
    }
    for transform in transforms.iter_mut() {
        for i in transform {
            if i.0 == node.index() {
                i.1 = parent_mat;
            }
        }
    }
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            meshes.push(Mesh::from_gltf(device, &primitive, buffers, parent_mat, mesh.index(), primitive.material().index(), node.skin().map(|s| s.index()))?);
        }
    }
    for node in node.children() {
        parse_node(node, parent_mat, meshes, buffers, device, layout, camera, lights, materials, transforms)?
    }

    Ok(())
}

