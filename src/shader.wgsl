struct Fragment {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@group(0) @binding(0)
var fluid_texture: texture_2d<f32>;
@group(0) @binding(1)
var fluid_sampler: sampler;

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> Fragment {
    var fragment: Fragment;
    fragment.clip_position = vec4(position, 0.0, 1.0);
    fragment.tex_coords = position / 2.0 + 0.5;
    return fragment;
}

@fragment
fn fs_main(fragment: Fragment) -> @location(0) vec4<f32> {
    let density = textureSample(fluid_texture, fluid_sampler, fragment.tex_coords).x;
    let color = pow(density, 2.2);
    return vec4(color, color, color, 1.0);
}
