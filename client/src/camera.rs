use anyhow::*;
use cgmath::{InnerSpace, Rotation3};
use wgpu::util::DeviceExt;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,

    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Camera {
    pub fn new(device: &wgpu::Device, size: &winit::dpi::PhysicalSize<u32>) -> Result<Self> {
        let mut camera_uniform = CameraUniform::new();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Ok(Self {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: size.width as f32 / size.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,

            camera_uniform,
            camera_buffer,
            bind_group_layout: camera_bind_group_layout,
            bind_group,
        })
    }

    /// Rotate the camera based on the input mouse movement. dx rotates around the Y axis,
    /// dy looks up-and-down.
    pub fn input(&mut self, dx: f64, dy: f64) {
        let dx = -0.25 * dx as f32;
        let dy = 0.25 * dy as f32;
        let at = self.target - self.eye;
        let at =
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Deg(dx)) * at;
        let at = cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_y().cross(at).normalize(),
            cgmath::Deg(dy),
        ) * at;
        self.target = self.eye + at;
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.camera_uniform
            .update_view_proj(self.build_view_projection_matrix(), &self.eye);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn at(&self) -> cgmath::Vector3<f32> {
        self.target - self.eye
    }

    pub fn set_position(&mut self, new_pos: &cgmath::Point3<f32>) {
        let at = self.at();
        self.eye = *new_pos + 1.7f32 * cgmath::Vector3::unit_y();
        self.target = self.eye + at;
    }

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_pos: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, view_proj: cgmath::Matrix4<f32>, eye: &cgmath::Point3<f32>) {
        self.view_proj = view_proj.into();
        self.view_pos = eye.to_homogeneous().into();
    }
}
