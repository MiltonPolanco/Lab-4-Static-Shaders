struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    time: f32,
    planet_type: u32,
    render_moon: u32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_position: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    var pos = model.position;
    var normal = model.normal;
    
    if (uniforms.render_moon == 1u) {
        pos = pos * 0.25;
        let orbit_radius = 2.2;
        let orbit_speed = 1.0;
        let orbit_angle = uniforms.time * orbit_speed;
        pos.x += cos(orbit_angle) * orbit_radius;
        pos.z += sin(orbit_angle) * orbit_radius;
    }
    
    out.model_position = model.position;
    

    if (uniforms.planet_type == 4u && uniforms.render_moon == 0u) {
        let ring_distance = length(pos.xz);
        if (ring_distance > 1.15 && ring_distance < 3.0) {
            if (abs(pos.y) < 0.2) {
                pos.y = pos.y * 0.008;
                let ring_wave = sin(ring_distance * 25.0 + uniforms.time * 0.5) * 0.015;
                pos.y += ring_wave;
            }
        }
    }
    
    let world_position = uniforms.model * vec4<f32>(pos, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = uniforms.view_proj * world_position;
    out.normal = normalize((uniforms.model * vec4<f32>(normal, 0.0)).xyz);
    
    let phi = atan2(normal.z, normal.x);
    let theta = acos(clamp(normal.y, -1.0, 1.0));
    out.uv = vec2<f32>(phi / (2.0 * 3.14159265359) + 0.5, theta / 3.14159265359);
    
    return out;
}

fn hash3(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

fn fbm3d(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    
    for (var i = 0; i < 6; i = i + 1) {
        value += amplitude * noise3d(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    return value;
}

fn moon_shader(model_pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let sphere_pos = normalize(model_pos);
    
    let base_color = vec3<f32>(0.65, 0.65, 0.65);
    let dark_color = vec3<f32>(0.35, 0.35, 0.35);
    let crater_color = vec3<f32>(0.25, 0.25, 0.25);
    
    let crater_noise = fbm3d(sphere_pos * 12.0);
    let crater_mask = smoothstep(0.3, 0.7, crater_noise);
    
    let detail = fbm3d(sphere_pos * 8.0);
    let fine_detail = noise3d(sphere_pos * 25.0) * 0.1;
    
    var color = mix(crater_color, dark_color, crater_mask);
    color = mix(color, base_color, detail * 0.7);
    color += fine_detail;
    
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);
    color = color * (0.3 + diffuse * 0.7);
    
    return color;
}

fn rocky_planet(model_pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let sphere_pos = normalize(model_pos);
    
    let base_color = vec3<f32>(0.4, 0.3, 0.25);
    let dark_color = vec3<f32>(0.2, 0.15, 0.1);
    
    let crater_noise = fbm3d(sphere_pos * 10.0);
    let crater_mask = smoothstep(0.4, 0.6, crater_noise);
    
    let mountain_noise = fbm3d(sphere_pos * 5.0);
    let mountain_color = mix(base_color, vec3<f32>(0.5, 0.45, 0.4), mountain_noise);
    
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    var color = mix(dark_color, mountain_color, crater_mask);
    color = color * (0.3 + diffuse * 0.7);
    
    return color;
}

fn gas_giant(model_pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let sphere_pos = normalize(model_pos);
    
    let band_freq = 12.0;
    
  
    let color1 = vec3<f32>(1.0, 0.88, 0.65);
    let color2 = vec3<f32>(0.95, 0.65, 0.40);
    let color3 = vec3<f32>(0.75, 0.50, 0.35);
    
    let turbulence = fbm3d(sphere_pos * 5.0 + vec3<f32>(uniforms.time * 0.3, 0.0, 0.0));
    let disturbed_y = sphere_pos.y + turbulence * 0.35;
    let band = sin(disturbed_y * band_freq + uniforms.time * 0.5);
    
    var color = mix(color1, color2, smoothstep(-0.5, 0.3, band));
    color = mix(color, color3, smoothstep(0.2, 0.8, band));
    
   
    let spot_center = vec3<f32>(0.3, -0.1, 0.85);
    let spot_dist = length(sphere_pos - spot_center);
    let spot = smoothstep(0.40, 0.05, spot_dist);
    let spot_color = vec3<f32>(0.98, 0.28, 0.18);
    color = mix(color, spot_color, spot * 0.95);
    
    let light_dir = normalize(vec3<f32>(1.0, 0.5, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);
    color = color * (0.5 + diffuse * 0.85);
    
    return color;
}

fn lava_planet(model_pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let sphere_pos = normalize(model_pos);
    
    let lava_flow = fbm3d(sphere_pos * 4.0 + vec3<f32>(uniforms.time * 0.5, uniforms.time * 0.3, 0.0));
    let cracks = fbm3d(sphere_pos * 8.0);
    let crack_mask = smoothstep(0.45, 0.55, cracks);
    
    let hot_color = vec3<f32>(1.0, 0.3, 0.0);
    let cool_color = vec3<f32>(0.2, 0.05, 0.0);
    let glow_color = vec3<f32>(1.0, 0.8, 0.0);
    
    let pulse = sin(uniforms.time * 2.0 + lava_flow * 10.0) * 0.5 + 0.5;
    
    var color = mix(cool_color, hot_color, lava_flow);
    color = mix(color, glow_color, crack_mask * pulse);
    color = color * (1.0 + crack_mask * 0.5);
    
    return color;
}

fn crystal_planet(model_pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let sphere_pos = normalize(model_pos);
    
    let crystal_base = fbm3d(sphere_pos * 8.0);
    let crystal_detail = fbm3d(sphere_pos * 16.0);
    let variation = noise3d(sphere_pos * 20.0) * 0.15;
    
    let time_shift = uniforms.time * 0.5;
    let hue_shift1 = sin(time_shift + sphere_pos.x * 8.0 + sphere_pos.y * 5.0) * 0.5 + 0.5;
    let hue_shift2 = cos(time_shift * 0.7 + sphere_pos.z * 6.0 + sphere_pos.y * 4.0) * 0.5 + 0.5;
    
   
    let color1 = vec3<f32>(0.4, 0.7, 1.0);
    let color2 = vec3<f32>(0.7, 0.4, 1.0);
    let color3 = vec3<f32>(0.4, 1.0, 0.8);
    let color4 = vec3<f32>(0.9, 0.5, 1.0);
    
    var color = mix(color1, color2, hue_shift1);
    color = mix(color, color3, crystal_base * 0.5);
    color = mix(color, color4, hue_shift2 * 0.3);
    color = color * (0.85 + crystal_detail * 0.3);
    color = color + vec3<f32>(variation);
    
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let view_dir = normalize(-model_pos);
    let reflect_dir = reflect(-light_dir, normal);
    let specular = pow(max(dot(view_dir, reflect_dir), 0.0), 64.0);
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    color = color * (0.5 + diffuse * 0.5);
    color = color + vec3<f32>(1.0) * specular * 0.7;
    color = color * 1.1;
    
    return color;
}

fn ringed_planet(model_pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let ring_distance = length(model_pos.xz);
    let sphere_pos = normalize(model_pos);
    
    if (ring_distance < 1.12) {
        let base_noise = fbm3d(sphere_pos * 4.0);
        let ocean_color = vec3<f32>(0.15, 0.45, 0.80);
        let land_color = vec3<f32>(0.28, 0.70, 0.40);
        
        var color = mix(ocean_color, land_color, smoothstep(0.42, 0.58, base_noise));
        
        let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
        let diffuse = max(dot(normal, light_dir), 0.0);
        color = color * (0.4 + diffuse * 0.8);
        
        return color;
    } else {

        let ring_band = fract((ring_distance - 1.15) * 18.0);
        
        let ring_color1 = vec3<f32>(0.98, 0.92, 0.80);
        let ring_color2 = vec3<f32>(0.90, 0.75, 0.60);
        let ring_color3 = vec3<f32>(0.70, 0.55, 0.45);
        let ring_color4 = vec3<f32>(0.45, 0.35, 0.30);
        
        var color: vec3<f32>;
        if (ring_band < 0.25) {
            color = mix(ring_color1, ring_color2, ring_band * 4.0);
        } else if (ring_band < 0.5) {
            color = mix(ring_color2, ring_color3, (ring_band - 0.25) * 4.0);
        } else if (ring_band < 0.75) {
            color = mix(ring_color3, ring_color4, (ring_band - 0.5) * 4.0);
        } else {
            color = mix(ring_color4, ring_color1, (ring_band - 0.75) * 4.0);
        }
        
        let shadow_factor = smoothstep(-0.2, 0.5, model_pos.x);
        color = color * (0.25 + shadow_factor * 0.75);
        
        let ring_detail = noise3d(vec3<f32>(ring_distance * 50.0, 0.0, uniforms.time * 0.1));
        color = color * (0.85 + ring_detail * 0.15);
        
        return color;
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec3<f32>;
    
    if (uniforms.render_moon == 1u) {
        color = moon_shader(in.model_position, in.normal, in.uv);
    } else {
        if (uniforms.planet_type == 0u) {
            color = rocky_planet(in.model_position, in.normal, in.uv);
        } else if (uniforms.planet_type == 1u) {
            color = gas_giant(in.model_position, in.normal, in.uv);
        } else if (uniforms.planet_type == 2u) {
            color = lava_planet(in.model_position, in.normal, in.uv);
        } else if (uniforms.planet_type == 3u) {
            color = crystal_planet(in.model_position, in.normal, in.uv);
        } else {
            color = ringed_planet(in.model_position, in.normal, in.uv);
        }
    }
    
    return vec4<f32>(color, 1.0);
}