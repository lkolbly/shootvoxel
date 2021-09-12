// Vertex shader

struct InstanceInput {
    [[location(5)]] model_matrix_0: vec4<f32>;
    [[location(6)]] model_matrix_1: vec4<f32>;
    [[location(7)]] model_matrix_2: vec4<f32>;
    [[location(8)]] model_matrix_3: vec4<f32>;

    [[location(9)]] normal_matrix_0: vec3<f32>;
    [[location(10)]] normal_matrix_1: vec3<f32>;
    [[location(11)]] normal_matrix_2: vec3<f32>;
};

[[block]]
struct CameraUniform {
    view_proj: mat4x4<f32>;
    view_pos: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: CameraUniform;

[[block]]
struct Light {
    position: vec3<f32>;
    color: vec3<f32>;
};
[[group(2), binding(0)]]
var<uniform> light: Light;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] world_position: vec3<f32>;
};

[[stage(vertex)]]
fn main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_1,
    );

    var out: VertexOutput;

    out.tex_coords = model.tex_coords;
    let world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    out.world_position = world_position.xyz;
    out.world_normal = normal_matrix * model.normal;
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}

[[stage(fragment)]]
fn colored(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    //return vec4<f32>(in.color, 1.0);
    let color: vec4<f32> =  textureSample(t_diffuse, s_diffuse, in.tex_coords);

    let ambient_strength = 0.1;
    let ambient_color = light.color * ambient_strength;

    let light_dir = normalize(light.position - in.world_position);
    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = light.color * diffuse_strength;

    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let half_dir = normalize(view_dir - light_dir);
    let reflect_dir = reflect(-light_dir, in.world_normal);
    let specular_strength = pow(max(dot(half_dir, reflect_dir), 0.0), 32.0);
    let specular_color = specular_strength * light.color;

    let result = (specular_color + diffuse_color + ambient_color) * color.xyz;

    return vec4<f32>(result, color.a);

    //let near = 0.1;
    //let far = 100.0;
    //let depth = textureSampleCompare(t_depth, s_depth, in.tex_coords, in.clip_position.w);
    //let r = (2.0 * near) / (far + near - depth * (far - near));
    //return vec4<f32>(vec3<f32>(r), 1.0);
}
