struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );

    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );

    var out: VertexOutput;
    out.position = vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
    out.uv = uv[in_vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let opacity: f32 = 0.7;
    let radius: f32 = 0.02; // Sutil rounding
    
    // Simple rounded corner logic in UV space
    let dist_x = min(in.uv.x, 1.0 - in.uv.x);
    let dist_y = min(in.uv.y, 1.0 - in.uv.y);
    
    if (dist_x < radius && dist_y < radius) {
        let dx = radius - dist_x;
        let dy = radius - dist_y;
        if (dx * dx + dy * dy > radius * radius) {
            discard;
        }
    }

    return vec4<f32>(0.0, 0.0, 0.0, opacity);
}
