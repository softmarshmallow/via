// A colour declared in linear RGB (a pack's, assets.md 2.6, or the loader's own), as ALBEDO.
//
// Forward+ and Mobile take ALBEDO as linear. The Compatibility renderer, which the sheet tool and
// the loader suite use (build.md 15.1), takes ALBEDO as sRGB-encoded and decodes it before lighting
// (measured with Godot 4.7: an ALBEDO of 0.25 under a white ambient of energy 0.5 reads back as
// sRGB 0.173, that is linear 0.0508 * 0.5). There a linear colour is encoded first, so every
// renderer lights the colour as declared; without it, (0.55, 0.42, 0.33) is drawn as
// (0.26, 0.15, 0.09), darker and of another hue. An image sampled with `source_color` needs none of
// this: Godot does the same for it on each renderer.

vec3 albedo_from_linear(vec3 c) {
#if CURRENT_RENDERER == RENDERER_COMPATIBILITY
	vec3 x = clamp(c, 0.0, 1.0);
	return mix(x * 12.92, 1.055 * pow(x, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), x));
#else
	return c;
#endif
}
