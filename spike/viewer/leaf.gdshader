shader_type spatial;
render_mode cull_disabled;
// Leaf cards from a tree recipe: a painted atlas, shaded by how deep the card sits in its crown.

uniform sampler2D leaf_tex : source_color, filter_linear_mipmap, repeat_disable;
uniform vec3 tint : source_color = vec3(1.0);
uniform vec3 bark : source_color = vec3(0.4, 0.33, 0.28);

varying float part;
varying vec3 itint;

void vertex() {
	part = CUSTOM0.w;
	itint = INSTANCE_CUSTOM.rgb;
	vec3 w = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	float lift = clamp(VERTEX.y / 9.0, 0.0, 1.0);
	float gust = sin(TIME * 1.1 + w.x * 0.07 + w.z * 0.05) + 0.5 * sin(TIME * 2.3 + w.x * 0.31);
	VERTEX.x += 0.06 * gust * lift;
	VERTEX.z += 0.04 * gust * lift;
}

void fragment() {
	float shade = mix(0.34, 1.0, COLOR.r);
	if (part > 0.0001) {
		vec4 t = texture(leaf_tex, UV);
		if (t.a < 0.42) {
			discard;
		}
		ALBEDO = t.rgb * tint * shade * itint;
	} else if (part < -0.5) {
		ALBEDO = bark;
	} else {
		ALBEDO = tint * 0.32 * shade * itint;
	}
	if (!FRONT_FACING) {
		NORMAL = -NORMAL;
	}
	ROUGHNESS = 0.9;
	SPECULAR = 0.08;
}
