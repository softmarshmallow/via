// A primitive or stand-in asset (spec/assets.md 6): its colour, times each instance's tint, which
// the MultiMesh carries as instance custom data (build.md 15.1).
shader_type spatial;

#include "res://addons/via_loader/linear.gdshaderinc"

uniform vec3 colour = vec3(0.8);   // linear, as declared (assets.md 2.6): no source_color, which would convert it again

varying float tint;

void vertex() {
	tint = INSTANCE_CUSTOM.r;
}

void fragment() {
	ALBEDO = albedo_from_linear(colour * tint);
	ROUGHNESS = 0.85;
}
