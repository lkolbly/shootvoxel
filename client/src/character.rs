use crate::instance::{Instance, InstanceBuffer};
use crate::model::{DrawModel, Model};
use cgmath::Rotation3;
use log::*;
use std::collections::HashMap;

const MAX_INSTANCES: usize = 256;

pub struct Character {
    instance_id: usize,
}

pub struct CharacterSet {
    model: Model,
    characters: HashMap<u32, Character>,
    instance_buffer: InstanceBuffer,
}

impl CharacterSet {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        let res_dir = std::path::Path::new(".").join("res");
        Self {
            model: Model::load(device, queue, layout, res_dir.join("cube.obj")).unwrap(),
            characters: HashMap::new(),
            instance_buffer: InstanceBuffer::new(device, MAX_INSTANCES),
        }
    }

    pub fn add(&mut self, id: u32, position: [f32; 3]) {
        if self.characters.len() >= MAX_INSTANCES {
            panic!("Instance overrun");
        }
        info!("Creating character at {:?}", position);
        let instance = Instance {
            position: position.into(),
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_z(),
                cgmath::Deg(0.0),
            ),
        };
        let instance_id = self.instance_buffer.instances.len();
        self.instance_buffer.instances.push(instance);
        self.characters.insert(id, Character { instance_id });
    }
}

pub trait DrawCharacterSet<'a> {
    fn draw_character_set(&mut self, queue: &'_ wgpu::Queue, charset: &'a mut CharacterSet);
}

impl<'a, 'b> DrawCharacterSet<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_character_set(&mut self, queue: &'_ wgpu::Queue, charset: &'b mut CharacterSet) {
        charset.instance_buffer.update(queue);
        self.draw_model_instanced(&charset.model, 0..charset.characters.len() as u32);
    }
}
