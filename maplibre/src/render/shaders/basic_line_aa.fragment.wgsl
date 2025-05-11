struct Output {
    @location(0) out_color: vec4<f32>,
};

@fragment
fn main(
    @location(0) v_color: vec4<f32>,
    @location(1) @interpolate(linear, center) v_normal: vec2<f32>,
    @location(2) line_width: f32,
    @builtin(position) position: vec4<f32>,
) -> Output {
//    let mag = length(v_normal);
//    if mag == 0 {
    return Output(v_color);
//    }

    // Apply line antialiasing
//    let feather = clamp(0.5 + (line_width - 10.0) / 10.0, 0.0, 0.95);

//    let blur: f32 = clamp((1 - mag) + feather, 0.0, 1.0);
//    let blur: f32 = 1.0;
//
//    return Output(v_color * blur);
}
