// A primitive or stand-in asset (spec/assets.md 6): its colour, times each instance's tint, which
// the MultiMesh carries as instance custom data (build.md 15.1).
shader_type spatial;

uniform vec3 colour : source_color = vec3(0.8);

varying float tint;

void vertex() {
	tint = INSTANCE_CUSTOM.r;
}

void fragment() {
	ALBEDO = colour * tint;
	ROUGHNESS = 0.85;
}
