use wgpu::*;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use gltf::Primitive;
use gltf::buffer::Data;
use anyhow::{Result, anyhow};
use glam::Mat4;
use std::cell::RefCell;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    weights: [f32; 4],
    joints: [u32; 4],
}

pub struct Mesh {
    pub index: usize,
    pub mat_index: Option<usize>,
    pub skin_index: Option<usize>,
    pub vertices: Buffer,
    pub indices: Buffer,
    pub length: u32,
    pub bind_group: Option<BindGroup>,
    pub transform_buffer: Option<Buffer>,
    pub joint_matrices_buffer: Option<Buffer>,
    pub matrix: RefCell<Mat4>,
}

impl Mesh {
    pub fn from_gltf(device: &Device, primitive: &Primitive, buffers: &Vec<Data>, matrix: Mat4, index: usize, mat_index: Option<usize>, skin_index: Option<usize>) -> Result<Self> {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        let positions = reader.read_positions().ok_or(anyhow!("Couldn't get positions"))?;
        let normals = reader.read_normals().ok_or(anyhow!("Couldn't get normals"))?;
        let weights = reader.read_weights(0).map(|i| i.into_f32()).into_iter().flatten().chain(std::iter::repeat([0.25, 0.25, 0.25, 0.25]));
        let joints = reader.read_joints(0).map(|i| i.into_u16()).into_iter().flatten().chain(std::iter::repeat([0, 0, 0, 0]));

        let indices_buf = reader.read_indices().ok_or(anyhow!("Couldn't get indices"))?.into_u32().collect::<Vec<_>>();

        let raw_vertices = positions.zip(normals).zip(weights).zip(joints).map(|(((p, n), w), j)| {
            Vertex {
                position: p,
                normal: n,
                weights: w,
                joints: [j[0] as u32, j[1] as u32, j[2] as u32, j[3] as u32],
            }
        }).collect::<Vec<_>>();

        let vertices = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&raw_vertices),
            usage: BufferUsage::VERTEX,
        });

        let length = indices_buf.len() as u32;
        let indices = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices_buf),
            usage: BufferUsage::INDEX,
        });

        let matrix = RefCell::new(matrix);

        Ok(Self {
            vertices,
            indices,
            length,
            index,
            mat_index,
            skin_index,
            matrix,
            bind_group: None,
            transform_buffer: None,
            joint_matrices_buffer: None,
        })
    }

    pub fn update_transforms(&self, queue: &Queue, matrix: Mat4) {
        self.matrix.replace(matrix);
        let normal_mat = matrix.inverse().transpose();

        queue.write_buffer(self.transform_buffer.as_ref().expect("Unbound mesh!"), 0, bytemuck::cast_slice(&[matrix, normal_mat]));
    }

    pub fn update_joints(&self, queue: &Queue, joints: &[Mat4]) {
        queue.write_buffer(self.joint_matrices_buffer.as_ref().expect("Unbound mesh!"), 0, bytemuck::cast_slice(&joints));
    }

    pub fn bind(&mut self, device: &Device, layout: &BindGroupLayout, joint_matrices: &[Mat4], material: &Buffer) {

        let matrix = *self.matrix.borrow();
        let normal_mat = matrix.inverse().transpose();

        let transform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[matrix, normal_mat]),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        let joint_matrices_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&joint_matrices),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: transform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: material.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: joint_matrices_buffer.as_entire_binding(),
                }
            ],
            label: None,
        }));
        self.transform_buffer = Some(transform_buffer);
        self.joint_matrices_buffer = Some(joint_matrices_buffer);
    }

    pub fn get_vertex_desc(&self) -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: InputStepMode::default(),
            attributes: &[
                VertexAttribute { // positions
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute { // normals
                    format: VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                },
                VertexAttribute { // weights
                    format: VertexFormat::Float32x4,
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 4]>()) as BufferAddress,
                    shader_location: 2,
                },
                VertexAttribute { // joints
                    format: VertexFormat::Uint32x4,
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 4]>() + std::mem::size_of::<[u32; 4]>()) as BufferAddress,
                    shader_location: 3,
                }
            ],
        }
    }
}

