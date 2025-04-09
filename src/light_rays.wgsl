#import bevy_sprite::{mesh2d_vertex_output::VertexOutput, mesh2d_view_bindings::globals}

@group(2) @binding(0) var base_color_texture: texture_2d<f32>;
@group(2) @binding(1) var base_color_sampler: sampler;
@group(2) @binding(2) var<uniform> translation: vec4<f32>;
@group(2) @binding(3) var<uniform> size: vec4<f32>;

fn rayStrength(
    raySource: vec2<f32>,
    rayRefDir: vec2<f32>,
    coord: vec2<f32>,
    seedA: f32,
    seedB: f32,
    speed: f32
) -> f32 {
    let sourceToCoord = coord - raySource;
    let cosAngle = dot(normalize(sourceToCoord), rayRefDir);
    return clamp(
        (0.45 + 0.15 * sin(cosAngle * seedA + globals.time * speed)) +
        (0.3  + 0.2 * cos(-cosAngle * seedB + globals.time * speed)),
        0.0,
        1.0
    );
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let coord = (mesh.world_position.xy / mesh.world_position.w) * 0.01;

    let rayPos1    = (vec2<f32>(-size.x / 1.2, size.x / 1.5) + translation.xy) * 0.01;
    let rayRefDir1 = normalize(vec2<f32>(1.0, 1.0));
    let raySeedA1  = 36.2;
    let raySeedB1  = 21.1;
    let raySpeed1  = 5.5;

    let rayPos2    = (vec2<f32>(size.x / 1.2, size.x / 1.5) + translation.xy) * 0.01;
    let rayRefDir2 = normalize(vec2<f32>(1.0, -1.0));
    let raySeedA2  = 22.4;
    let raySeedB2  = 18.0;
    let raySpeed2  = 5.1;

    let rays1 = vec4<f32>(1.0, 1.0, 1.0, 1.0) * rayStrength(rayPos1, rayRefDir1, coord, raySeedA1, raySeedB1, raySpeed1);
    let rays2 = vec4<f32>(1.0, 1.0, 1.0, 1.0) * rayStrength(rayPos2, rayRefDir2, coord, raySeedA2, raySeedB2, raySpeed2);

    var fragColor = rays1 * 0.7 + rays2 * 0.6;

    let contrastFactor = 2.0;
    fragColor.x = pow(fragColor.x, contrastFactor);
    fragColor.y = pow(fragColor.y, contrastFactor);
    fragColor.z = pow(fragColor.z, contrastFactor);

    return fragColor * textureSample(base_color_texture, base_color_sampler, mesh.uv);
}
