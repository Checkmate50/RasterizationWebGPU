use wgpu::*;
use wgpu::util::DeviceExt;
use glam::{Vec2, Vec3, Vec4};
use core::ops::{Mul, Add};
use std::f32::consts::PI;

pub struct Sky {
    pub bind_group: BindGroup,
    pub layout: BindGroupLayout,
}

struct Vec5 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    v: f32,
}

impl Vec5 {
    fn new(x: f32, y:f32, z: f32, w: f32, v: f32) -> Self {
        Self { x, y, z, w, v }
    }
}

impl Mul<f32> for Vec5 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            w: self.w * rhs,
            v: self.v * rhs,
        }
    }
}

impl Add for Vec5 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
            v: self.v + rhs.v,
        }
    }
}

struct Mat5x2(Vec5, Vec5);

impl Mul<Vec2> for Mat5x2 {
    type Output = Vec5;

    fn mul(self, rhs: Vec2) -> Self::Output {
        self.0 * rhs.x + self.1 * rhs.y
    }
}

struct Mat4x3(Vec4, Vec4, Vec4);

impl Mul<Vec3> for Mat4x3 {
    type Output = Vec4;

    fn mul(self, rhs: Vec3) -> Self::Output {
        self.0 * rhs.x + self.1 * rhs.y + self.2 * rhs.z
    }
}

fn y_z(theta: f32, t: f32) -> f32 {
    let chi = (4.0 / 9.0 - t / 120.0) * (PI - 2.0 * theta);
    (4.0453 * t - 4.9710) * f32::tan(chi) - 0.2155 * t + 2.4192
}

fn __z(theta: f32, t: f32, m: Mat4x3) -> f32 {
    let vt = Vec3::new(t * t, t, 1.0);
    let vth = Vec4::new(theta * theta * theta, theta * theta, theta, 1.0);
    (m * vt).dot(vth)
}

impl Sky {
    pub fn new(theta_sun: f32, turbidity: f32, device: &Device) -> Sky {

        let c_y = Mat5x2(
            Vec5::new(0.1787, -0.3554, -0.0227, 0.1206, -0.0670),
            Vec5::new(-1.4630, 0.4275, 5.3251, -2.5771, 0.3703),
            );
        let cx = Mat5x2(
            Vec5::new(-0.0193, -0.0665, -0.0004, -0.0641, -0.0033),
            Vec5::new(-0.2592, 0.0008, 0.2125, -0.8989, 0.0452),
            );
        let cy = Mat5x2( 
            Vec5::new(-0.0167, -0.0950, -0.0079, -0.0441, -0.0109),
            Vec5::new(-0.2608, 0.0092, 0.2102, -1.6537, 0.0529),
            );

        let mx = Mat4x3(
            Vec4::new(0.0017, -0.0037,  0.0021,  0.0000),
            Vec4::new(-0.0290,  0.0638, -0.0320,  0.0039),
            Vec4::new(0.1169, -0.2120,  0.0605,  0.2589),
            );

        let my = Mat4x3(
            Vec4::new(0.0028, -0.0061,  0.0032,  0.0000),
            Vec4::new(-0.0421,  0.0897, -0.0415,  0.0052),
            Vec4::new(0.1535, -0.2676,  0.0667,  0.2669),
            );

        let p_y = c_y * Vec2::new(turbidity, 1.0);
        let px = cx * Vec2::new(turbidity, 1.0);
        let py = cy * Vec2::new(turbidity, 1.0);

        let y_z = y_z(theta_sun, turbidity);
        let xz = __z(theta_sun, turbidity, mx);
        let yz = __z(theta_sun, turbidity, my);

        let a = Vec3::new(p_y.x, px.x, py.x).extend(0.0);
        let b = Vec3::new(p_y.y, px.y, py.y).extend(0.0);
        let c = Vec3::new(p_y.z, px.z, py.z).extend(0.0);
        let d = Vec3::new(p_y.w, px.w, py.w).extend(0.0);
        let e = Vec3::new(p_y.v, px.v, py.v).extend(0.0);
        let zenith = Vec3::new(y_z, xz, yz).extend(0.0);

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[a, b, c, d, e, zenith, Vec4::new(theta_sun, 0.0, 0.0, 0.0)]),
            usage: BufferUsage::UNIFORM,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: None,
        });


        Self {
            bind_group, 
            layout,
        }
    }
}
