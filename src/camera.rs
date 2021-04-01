use glam::{Vec3, Mat3, Mat4};
use wgpu::*;
use wgpu::util::DeviceExt;
use gltf::camera::Projection;

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub near: f32,
    pub far: f32,
    pub aspect: f32,
    pub vfov: f32,
    pub mat_buffer: Buffer,
    pub pos_buffer: Buffer,
    pub bind_group: BindGroup,
    pub layout: BindGroupLayout,
}

impl Camera {
    pub fn new(device: &Device, eye: Vec3, target: Vec3, up: Vec3, near: f32, far: f32, aspect: f32, vfov: f32) -> Self {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: None,
        });

        let view_mat = Mat4::look_at_rh(eye, target, up);
        let proj_mat = Mat4::perspective_rh(vfov, aspect, near, far);
        let mat_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[view_mat, proj_mat]),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        let pos_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&eye),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: mat_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: pos_buffer.as_entire_binding(),
                }
            ],
            label: None,
        });
        
        Self {
            mat_buffer,
            pos_buffer,
            eye,
            target,
            up,
            near,
            far,
            aspect,
            bind_group,
            layout,
            vfov
        }
    }

    pub fn from_gltf(device: &Device, proj: Projection, mat: Mat4) -> Self {
        if let Projection::Perspective(p) = proj {

            let eye = mat.transform_point3(Vec3::new(0.0, 0.0, 0.0));
            let target = mat.transform_point3(Vec3::new(0.0, 0.0, -1.0));
            let up = mat.transform_vector3(Vec3::new(0.0, 1.0, 0.0));

            let near = p.znear();
            let far = p.zfar().unwrap_or(50.0);
            let aspect = p.aspect_ratio().unwrap_or(1.333);
            let vfov = p.yfov();

            Self::new(device, eye, target, up, near, far, aspect, vfov)
        } else {
            unimplemented!("Orthographic cameras are unsupported");
        }
    }

    pub fn update(&mut self, queue: &Queue, mat: Mat3) {
        self.eye = mat * self.eye;
        self.target = mat * self.target;
        self.up = mat * self.up;
        let u_view = self.get_view_mat();
        let u_proj = self.get_proj_mat();
        queue.write_buffer(&self.mat_buffer, 0, bytemuck::cast_slice(&[u_view, u_proj]));
        queue.write_buffer(&self.pos_buffer, 0, bytemuck::bytes_of(&self.eye));
    }

    pub fn get_view_mat(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    pub fn get_proj_mat(&self) -> Mat4 {
        Mat4::perspective_rh(self.vfov, self.aspect, self.near, self.far)
    }
}

