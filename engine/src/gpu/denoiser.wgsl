fn guassian_weight(x: vec3<f32>, y: vec3<f32>, sigma: f32) -> f32 {
    let dist_sq = dot(x - y, x - y);
    return exp(-dist_sq / (2.0 * sigma * sigma));
}

const SIGMA_SPACE: f32 = 1.0;
const KERNAL_SIZE: i32 = 10;

fn bilateral_filter(center_color: vec3<f32>, center_coords: vec3<i32>) -> vec4<f32> {
    var result = vec3(0.0);
    var weight_sum = 0.0;
    
    var i: i32 = -KERNAL_SIZE;
    while i <= KERNAL_SIZE {
        var j: i32 = -KERNAL_SIZE;
        while j <= KERNAL_SIZE {
            let current_coords: vec3<i32> = center_coords + vec2(i, j);
            let current_color: vec3<f32> = texelFetch(inputTexture, current_coords, 0).rgb;
            
            let color_weight: f32 = guassian_weight(center_color, current_color, vec3(1.0));
            let spatial_weight: f32 = guassian_weight(vec3(center_coords), vec3(current_coords), SIGMA_SPACE);
            let weight: f32 = color_weight * spatial_weight;
            
            result += current_color * weight;
            weight_sum += weight;
            
            j += 1;
        }
        i += 1;
    }
    
    return vec4(result / weight_sum, 1.0);
}
