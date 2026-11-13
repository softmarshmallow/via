// The ground of one chunk (spec/build.md 15.1). Material weights come as one Texture2DArray,
// one R8 layer per material the chunk names; the shader loops over the chunk's count, so there is
// no fixed number of materials. `params` holds one column per material: row 0 its colour (linear,
// alpha 1 when it has an image), row 1 its repeat in metres. A post where every weight is 0 holds
// no material; the loader then draws its own default colour.
shader_type spatial;
render_mode cull_back;

uniform sampler2DArray weights : filter_linear, repeat_disable;
uniform sampler2D params : filter_nearest, repeat_disable;
uniform sampler2DArray albedo : source_color, filter_linear_mipmap, repeat_enable;
uniform int count = 0;
uniform vec3 default_colour = vec3(0.6, 0.6, 0.6);

varying vec2 plan;

void vertex() {
	vec3 w = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	plan = vec2(w.x, -w.z);   // world x east, y north
}

void fragment() {
	vec3 acc = vec3(0.0);
	float total = 0.0;
	for (int i = 0; i < count; i++) {
		float w = texture(weights, vec3(UV, float(i))).r;
		if (w <= 0.0) {
			continue;
		}
		vec4 p = texelFetch(params, ivec2(i, 0), 0);
		vec3 c = p.rgb;
		if (p.a > 0.5) {
			float rep = texelFetch(params, ivec2(i, 1), 0).r;
			c = texture(albedo, vec3(plan / rep, float(i))).rgb;
		}
		acc += w * c;
		total += w;
	}
	ALBEDO = total > 0.001 ? acc / total : default_colour;
	ROUGHNESS = 0.95;
}
