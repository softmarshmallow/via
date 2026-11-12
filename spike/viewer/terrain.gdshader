shader_type spatial;
// The ground: a flat grid lifted by the height field, coloured by the material weights.

#include "res://ground.gdshaderinc"

uniform sampler2D height_tex : filter_linear, repeat_disable;
uniform sampler2D hole_tex : filter_nearest, repeat_disable;
uniform float res = 0.5;

varying vec3 wpos;

float hgt(vec2 xy) {
	return textureLod(height_tex, huv(xy), 0.0).r;
}

// The surface normal from the field itself, in engine axes.
vec3 field_normal(vec2 xy) {
	float hx = hgt(xy + vec2(res, 0.0)) - hgt(xy - vec2(res, 0.0));
	float hy = hgt(xy + vec2(0.0, res)) - hgt(xy - vec2(0.0, res));
	return normalize(vec3(-hx / (2.0 * res), 1.0, hy / (2.0 * res)));
}

void vertex() {
	vec3 w = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec2 xy = vec2(w.x, -w.z);
	float h = hgt(xy);
	VERTEX.y = h - MODEL_MATRIX[3].y;      // the node may stand at any height; the ground does not move
	NORMAL = field_normal(xy);
	wpos = vec3(w.x, h, w.z);
}

void fragment() {
	vec2 xy = vec2(wpos.x, -wpos.z);
	vec2 uv = huv(xy);
	if (texture(hole_tex, uv).r > 0.5) {
		discard;      // a volume stands in for the ground here
	}
	// a per-pixel normal from the field keeps slopes crisp between vertices
	vec3 n = field_normal(xy);
	vec4 a = texture(w0, uv);
	vec4 b = texture(w1, uv);
	float w[GROUND_LAYERS] = { a.r, a.g, a.b, a.a, b.r, b.g, b.b, b.a };
	float far = ground_far(wpos, CAMERA_POSITION_WORLD);
	float steep = smoothstep(0.82, 0.58, n.y);
	ALBEDO = ground_blend(w, wpos, n, far, steep, steep, dFdx(wpos.xz), dFdy(wpos.xz));
	ROUGHNESS = 0.94;
	SPECULAR = 0.1;
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
