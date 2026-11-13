extends RefCounted
## The sheet tool's light and environment (spec/build.md 15.1): one sun and one environment, the
## same for both builds, so a pair differs only by build. Shared with the exposure check
## (tests/test_exposure.gd), which renders known colours under exactly this rig.
##
## Exposure is set here, never in the colours (a pack declares linear albedo, assets.md 2.6; the
## loader's shaders hand it to the renderer as such, linear.gdshaderinc). The tone map is linear, the
## ambient neutral and the sky not reflected, so a lit face is its albedo times one factor for all
## three channels: hue and channel ratios are kept as declared. Shadow keeps AMBIENT_ENERGY of its
## albedo, so shadowed ground is dark, not black. The sun's share is not proportional to its
## energy in Godot 4.7's Compatibility renderer (measured at N.L 0.5: energy 0.08, 0.16 and 0.32
## give factors 0.40, 0.54 and 0.76 with this ambient), so SUN_ENERGY is set by a grey card, not a
## formula: albedo 0.5 at N.L 0.5 reads linear 0.27 (sRGB 142), mid-scale, and a near-white of 0.8
## full on to the sun reads 0.62 (sRGB 207), so no pack colour clips. tests/test_exposure.gd checks
## both.

const LIGHT_BEARING := 225.0
const LIGHT_ELEVATION := 45.0
const SUN_ENERGY := 0.16
const AMBIENT_ENERGY := 0.18
## Neutral, so ambient light changes no hue.
const AMBIENT_COLOUR := Color(1, 1, 1)


static func g(x: float, y: float, z: float) -> Vector3:
	## World (x east, y north, z up) to Godot (X, Y, Z) = (x, z, -y) (build.md 15.1).
	return Vector3(x, z, -y)


static func to_sun() -> Vector3:
	## The unit vector toward the sun, in Godot axes.
	var b := deg_to_rad(LIGHT_BEARING)
	var e := deg_to_rad(LIGHT_ELEVATION)
	return g(sin(b) * cos(e), cos(b) * cos(e), sin(e))


static func rig(vp: Node) -> void:
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = Sky.new()
	var sky := ProceduralSkyMaterial.new()
	# Below the horizon is past the extent's edge, where nothing is drawn: shown as the sky's own
	# horizon, never the default's brown, which reads at eye height as ground or mud.
	sky.ground_horizon_color = sky.sky_horizon_color
	sky.ground_bottom_color = sky.sky_horizon_color
	env.sky.sky_material = sky
	# Light on surfaces is set by the two energies alone: a fixed neutral ambient, no reflections of
	# the sky, and a linear tone map, so what is drawn is proportional to what is declared.
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = AMBIENT_COLOUR
	env.ambient_light_energy = AMBIENT_ENERGY
	env.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	env.tonemap_exposure = 1.0
	var we := WorldEnvironment.new()
	we.environment = env
	vp.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.light_energy = SUN_ENERGY
	sun.transform = Transform3D(Basis.looking_at(-to_sun(), Vector3.UP), Vector3.ZERO)
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 250.0
	vp.add_child(sun)
