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
    let density = clamp(textureSample(fluid_texture, fluid_sampler, fragment.tex_coords).x, 0.0, 1.0);
    return vec4(density, density, density, 1.0);
}
