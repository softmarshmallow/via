// The ground's materials, shared by the terrain and the meshed volumes so that both draw the
// same ground: up to eight layers, blended by the weights the compiler painted.

#define GROUND_LAYERS 8

uniform sampler2D w0 : filter_linear, repeat_disable;
uniform sampler2D w1 : filter_linear, repeat_disable;
uniform sampler2DArray ground : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform vec4 extent;
uniform vec2 posts;
uniform float tile[GROUND_LAYERS];

// Where a world position falls in the field textures, post centre to texel centre.
vec2 huv(vec2 xy) {
	vec2 f = (xy - extent.xy) / (extent.zw - extent.xy);
	return (f * (posts - 1.0) + 0.5) / posts;
}

// How much of the wide, low-detail sample to use: none up close, most of it far away.
float ground_far(vec3 p, vec3 eye) {
	return smoothstep(20.0, 150.0, length(eye - p));
}

// One layer's colour. `steep` says whether side projections are worth sampling, `side` how much
// of them to use.
vec3 ground_layer(int i, vec3 p, vec3 n, float far, float steep, float side, vec2 gx, vec2 gy) {
	// explicit gradients, so the samples can sit behind branches without mip seams
	float s = 1.0 / tile[i];
	float layer = float(i);
	vec3 top = textureGrad(ground, vec3(p.xz * s, layer), gx * s, gy * s).rgb;
	if (far > 0.02) {
		vec3 wide = textureGrad(ground, vec3(p.xz * s * 0.17 + 0.37, layer), gx * s * 0.17, gy * s * 0.17).rgb;
		top = mix(top, wide, far * 0.8);
	}
	if (steep > 0.02) {
		vec3 bx = textureGrad(ground, vec3(p.zy * s, layer), gx * s, gy * s).rgb;
		vec3 bz = textureGrad(ground, vec3(p.xy * s, layer), gx * s, gy * s).rgb;
		vec3 an = abs(n);
		top = mix(top, mix(bz, bx, an.x / (an.x + an.z + 0.0001)), side);
	}
	return top;
}

// The weighted blend of all layers at a point. `gx` and `gy` are the screen-space gradients of p.xz.
vec3 ground_blend(float w[GROUND_LAYERS], vec3 p, vec3 n, float far, float steep, float side, vec2 gx, vec2 gy) {
	vec3 col = vec3(0.0);
	float total = 0.0;
	for (int i = 0; i < GROUND_LAYERS; i++) {
		if (w[i] > 0.01) {
			col += w[i] * ground_layer(i, p, n, far, steep, side, gx, gy);
			total += w[i];
		}
	}
	col /= max(total, 0.001);
	// broad tonal drift so large areas are not one flat colour
	float drift = sin(p.x * 0.021 + sin(p.z * 0.017) * 2.0) * sin(p.z * 0.019 + 1.3);
	return col * (1.0 + 0.07 * drift);
}
