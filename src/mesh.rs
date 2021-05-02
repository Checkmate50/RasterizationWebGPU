use wgpu::*;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use gltf::Primitive;
use gltf::buffer::Data;
use anyhow::{Result, anyhow};
use glam::Mat4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

pub struct Mesh {
    pub index: usize,
    pub vertices: Buffer,
    pub indices: Buffer,
    pub length: u32,
    pub bind_group: BindGroup,
    pub mat_buffer: Buffer,
}

impl Mesh {
    pub fn from_obj(device: &Device, mesh: &tobj::Mesh, layout: &BindGroupLayout, material: &Buffer) -> Self {

        let raw_vertices = mesh.positions.array_chunks::<3>().zip(mesh.normals.array_chunks::<3>()).map(|(v, n)| {
            Vertex {
                position: *v,
                normal: *n,
            }
        }).collect::<Vec<_>>();

        let vertices = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&raw_vertices),
            usage: BufferUsage::VERTEX,
        });

        let length = mesh.indices.len() as u32;
        let indices = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: BufferUsage::INDEX,
        });

        let (bind_group, mat_buffer) = bind_group_from_mat(device, layout, Mat4::IDENTITY, material);

        Self {
            vertices,
            indices,
            length,
            bind_group,
            mat_buffer,
            index: 0,
        }
    }

    pub fn from_gltf(device: &Device, primitive: &Primitive, buffers: &Vec<Data>, mat: Mat4, layout: &BindGroupLayout, material: &Buffer, index: usize) -> Result<Self> {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
        let positions_buf = reader.read_positions().ok_or(anyhow!("Couldn't get positions"))?;
        let normals_buf = reader.read_normals().ok_or(anyhow!("Couldn't get normals"))?;
        let indices_buf = reader.read_indices().ok_or(anyhow!("Couldn't get indices"))?.into_u32().collect::<Vec<_>>();

        let raw_vertices = positions_buf.zip(normals_buf).map(|(v, n)| {
            Vertex {
                position: v,
                normal: n,
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

        let (bind_group, mat_buffer) = bind_group_from_mat(device, layout, mat, material);

        Ok(Self {
            vertices,
            indices,
            length,
            bind_group,
            mat_buffer,
            index,
        })
    }

    pub fn update_matrices(&self, queue: &Queue, matrix: Mat4) {
        let normal_mat = matrix.inverse().transpose();

        queue.write_buffer(&self.mat_buffer, 0, bytemuck::cast_slice(&[matrix, normal_mat]));
    }

    pub fn get_vertex_desc(&self) -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: InputStepMode::default(),
            attributes: &[
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                }
            ],
        }
    }
}

fn bind_group_from_mat(device: &Device, layout: &BindGroupLayout, matrix: Mat4, material: &Buffer) -> (BindGroup, Buffer) {

    let normal_mat = matrix.inverse().transpose();

    let transform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&[matrix, normal_mat]),
        usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
    });

    (device.create_bind_group(&BindGroupDescriptor {
        layout: &layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: material.as_entire_binding(),
            }
        ],
        label: None,
    }), transform_buffer)
}

