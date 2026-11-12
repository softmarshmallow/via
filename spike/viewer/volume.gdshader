shader_type spatial;
// A meshed volume: ground that is more than a surface. It borrows the terrain's materials.

#include "res://ground.gdshaderinc"

uniform int rock_layer = 0;
uniform vec3 rock_a : source_color = vec3(1.0);
uniform vec3 rock_b : source_color = vec3(1.0);
uniform float rock_tint = 0.0;
uniform float band_m = 3.0;
uniform float sea_level = -1000.0;

varying vec3 wpos;
varying vec3 wnrm;
varying float sky;

void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	wnrm = normalize(NORMAL);
	sky = COLOR.r;
}

void fragment() {
	vec2 xy = vec2(wpos.x, -wpos.z);
	vec2 uv = huv(xy);
	vec3 n = normalize(wnrm);
	vec4 a = texture(w0, uv);
	vec4 b = texture(w1, uv);
	float far = ground_far(wpos, CAMERA_POSITION_WORLD);
	// tops take the surrounding ground; faces and ceilings are rock
	float steep = smoothstep(0.80, 0.55, n.y);
	a = mix(a, vec4(0.0), steep);
	b = mix(b, vec4(0.0), steep);
	float w[GROUND_LAYERS] = { a.r, a.g, a.b, a.a, b.r, b.g, b.b, b.a };
	w[rock_layer] += steep;
	// side projections fade out again where the face turns flat, up or down
	float side = steep * smoothstep(0.95, 0.6, abs(n.y));
	vec3 col = ground_blend(w, wpos, n, far, steep, side, dFdx(wpos.xz), dFdy(wpos.xz));
	col *= mix(0.55, 1.0, smoothstep(sea_level - 0.6, sea_level + 2.2, wpos.y));      // wet at the waterline
	if (rock_tint > 0.0) {      // layered rock: the texture keeps its detail, the beds bring the colour
		float lum = dot(col, vec3(0.3, 0.5, 0.2));
		// beds of uneven thickness and hardness, gently warped, with darker streaks running down the face
		float warp = 1.6 * sin(wpos.x * 0.043 + wpos.z * 0.031) + 0.7 * sin(wpos.x * 0.11 - wpos.z * 0.09);
		float y = wpos.y * 6.2832 / band_m + warp;
		float bed = 0.5 + 0.30 * sin(y) + 0.22 * sin(y * 2.63 + 1.3) + 0.14 * sin(y * 6.1 + 4.0) + 0.08 * sin(y * 13.7);
		bed = smoothstep(0.25, 0.75, bed);
		float streak = 0.5 + 0.5 * sin(wpos.x * 0.9 + wpos.z * 0.7 + 3.0 * sin(wpos.y * 0.08));
		bed = clamp(bed - 0.18 * smoothstep(0.6, 1.0, streak), 0.0, 1.0);
		vec3 tinted = mix(rock_a, rock_b, bed) * (0.45 + 2.2 * lum);
		col = mix(col, tinted, rock_tint * steep);
	}
	col *= mix(0.07, 1.0, smoothstep(0.0, 0.55, sky));      // rock that sees no sky is dark
	ALBEDO = col;
	ROUGHNESS = 0.94;
	SPECULAR = 0.1;
	NORMAL = normalize((VIEW_MATRIX * vec4(n, 0.0)).xyz);
}
