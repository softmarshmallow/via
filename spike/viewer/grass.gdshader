shader_type spatial;
render_mode cull_disabled;
// Blades take the ground's colour at their root, so the field of grass and the ground read as one.

uniform vec3 tip_gain = vec3(1.25, 1.35, 0.95);
uniform float wind = 1.0;

varying float up;

void vertex() {
	up = UV.y;
	vec3 w = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float phase = INSTANCE_CUSTOM.r * 6.28;
	float gust = sin(TIME * 1.4 + w.x * 0.11 + w.z * 0.07) * 0.6 + sin(TIME * 2.7 + phase + w.x * 0.5) * 0.4;
	vec3 push = vec3(0.09, 0.0, 0.05) * gust * wind * up * up;
	VERTEX += (inverse(MODEL_MATRIX) * vec4(push, 0.0)).xyz;
	NORMAL = normalize((inverse(MODEL_MATRIX) * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
}

void fragment() {
	vec3 root = COLOR.rgb;
	ALBEDO = mix(root * 0.82, root * tip_gain, up);
	ROUGHNESS = 0.85;
	SPECULAR = 0.12;
	NORMAL = normalize((VIEW_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
}
