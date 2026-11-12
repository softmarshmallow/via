shader_type spatial;
// Every placed asset: its own texture and colour, times the instance tint.

uniform sampler2D albedo_tex : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform vec4 albedo_col : source_color = vec4(1.0);
uniform float scissor = 0.0;
uniform float rough = 0.9;

varying vec3 tint;

void vertex() {
	tint = INSTANCE_CUSTOM.rgb;
}

void fragment() {
	vec4 t = texture(albedo_tex, UV) * albedo_col;
	if (scissor > 0.0 && t.a < scissor) {
		discard;
	}
	ALBEDO = t.rgb * tint;
	ROUGHNESS = rough;
	SPECULAR = 0.15;
}
