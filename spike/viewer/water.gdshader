shader_type spatial;
render_mode blend_mix, cull_back, depth_draw_always;
// Water is a field too: a surface height, drawn only where it lies above the ground.

uniform sampler2D height_tex : filter_linear, repeat_disable;
uniform sampler2D water_tex : filter_linear, repeat_disable;
uniform vec4 extent;
uniform vec2 posts;
uniform vec3 shallow : source_color = vec3(0.20, 0.47, 0.46);
uniform vec3 deep : source_color = vec3(0.03, 0.13, 0.20);
uniform float surf = 0.0;

varying vec3 wpos;

vec2 huv(vec2 xy) {
	vec2 f = (xy - extent.xy) / (extent.zw - extent.xy);
	return (f * (posts - 1.0) + 0.5) / posts;
}

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

float ripple(vec2 p) {
	return noise(p * 0.9 + vec2(TIME * 0.25, 0.0)) * 0.6 + noise(p * 2.3 - vec2(0.0, TIME * 0.35)) * 0.4;
}

void vertex() {
	vec3 w = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float lvl = textureLod(water_tex, huv(vec2(w.x, -w.z)), 0.0).r;
	VERTEX.y = lvl;
	wpos = vec3(w.x, lvl, w.z);
}

void fragment() {
	vec2 xy = vec2(wpos.x, -wpos.z);
	vec2 uv = huv(xy);
	float depth = textureLod(water_tex, uv, 0.0).r - textureLod(height_tex, uv, 0.0).r;
	if (depth <= 0.0) {
		discard;
	}
	float e = 0.35;
	float r0 = ripple(wpos.xz);
	vec3 n = normalize(vec3((r0 - ripple(wpos.xz + vec2(e, 0.0))) * 0.22, 1.0, (r0 - ripple(wpos.xz + vec2(0.0, e))) * 0.22));
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
	ALBEDO = mix(shallow, deep, smoothstep(0.0, 2.6, depth));
	ALPHA = mix(0.5, 0.97, smoothstep(0.0, 1.4, depth)) * smoothstep(0.0, 0.06, depth);
	if (surf > 0.0) {      // foam where the water is shallow, breaking in moving bands
		float band = noise(wpos.xz * 0.35 + vec2(TIME * 0.12, 0.0)) * 0.6 + noise(wpos.xz * 1.3 - vec2(0.0, TIME * 0.2)) * 0.4;
		float reach = smoothstep(1.3 * surf, 0.0, depth);
		float lines = smoothstep(0.80, 0.92, noise(vec2(wpos.x * 0.05, wpos.z * 0.6 + TIME * 0.15)) * 0.6 + band * 0.5) * smoothstep(7.0 * surf, 1.0, depth);
		float foam = smoothstep(0.70 - 0.45 * reach, 0.84 - 0.45 * reach, band) * reach + 0.6 * lines + smoothstep(0.3, 0.0, depth);
		foam = clamp(foam, 0.0, 1.0);
		ALBEDO = mix(ALBEDO, vec3(0.92, 0.95, 0.96), foam);
		ALPHA = max(ALPHA, foam * 0.95);
	}
	ROUGHNESS = 0.05;
	SPECULAR = 0.7;
}
