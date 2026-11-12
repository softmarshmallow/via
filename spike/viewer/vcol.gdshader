shader_type spatial;
// Geometry that carries its colour in its vertices.

void fragment() {
	ALBEDO = COLOR.rgb;
	ROUGHNESS = 0.92;
	SPECULAR = 0.1;
}
