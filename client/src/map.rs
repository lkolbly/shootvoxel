use crate::instance::{Instance, InstanceBuffer};
use crate::model::{DrawModel, Model};
use cgmath::Rotation3;
use log::*;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::io::Read;

#[derive(Deserialize)]
pub struct Voxel {
    x: u16,
    y: u16,
    z: u16,
}

pub struct Map {
    voxel_model: Model,
    voxels: Vec<Voxel>,
    instance_buffer: InstanceBuffer,
}

impl Map {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        let res_dir = std::path::Path::new(".").join("res");

        let mut voxels = vec![];
        let mut f = std::fs::File::open("map.bin").unwrap();
        let mut len = [0; 4];
        f.read_exact(&mut len).unwrap();
        let len = u32::from_le_bytes(len);
        for _ in 0..len {
            let mut voxel = [0; 6]; // TODO: Don't hardcode this voxel data length
            f.read_exact(&mut voxel).unwrap();
            let voxel: Voxel = bincode::deserialize(&voxel).unwrap();
            voxels.push(voxel);
        }

        let mut buf = InstanceBuffer::new(device, voxels.len());
        for (i, voxel) in voxels.iter().enumerate() {
            buf.instances.push(Instance {
                position: cgmath::Vector3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                rotation: cgmath::Quaternion::from_axis_angle(
                    cgmath::Vector3::unit_z(),
                    cgmath::Deg(0.0),
                ),
            });
            buf.instances[i].position = cgmath::Vector3 {
                x: voxel.x as f32 * 0.2,
                y: voxel.y as f32 * 0.2,
                z: voxel.z as f32 * 0.2,
            };
        }
        buf.update(queue);

        Self {
            voxel_model: Model::load(device, queue, layout, res_dir.join("voxel.obj")).unwrap(),
            voxels: voxels,
            instance_buffer: buf,
        }
    }
}

pub trait DrawMap<'a> {
    fn draw_map(&mut self, queue: &'_ wgpu::Queue, map: &'a mut Map);
}

impl<'a, 'b> DrawMap<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_map(&mut self, queue: &'_ wgpu::Queue, map: &'b mut Map) {
        map.instance_buffer.update(queue);
        self.set_vertex_buffer(1, map.instance_buffer.instance_buffer.slice(..));
        self.draw_model_instanced(&map.voxel_model, 0..map.voxels.len() as u32);
    }
}
