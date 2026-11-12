extends RefCounted
## Sky, sun and environment. A world's `look` table may override the colours and energies.

const WorldData := preload("res://world_data.gd")

## Shots look far; walking keeps shadows near so they stay sharp and cheap.
const SHADOW_REACH_SHOTS_M := 420.0
const SHADOW_REACH_WALK_M := 170.0
## The sun is a direction; it only needs to stand well outside the world, aimed at its middle.
const SUN_AIM_HEIGHT_M := 40.0
const SUN_DISTANCE_M := 600.0


static func build(world: WorldData, stage: Node3D, shots: bool) -> void:
	var look := world.look
	var winter: bool = look.get("sky", "summer") == "winter"
	var haze := float(look.get("haze", 0.5))
	var sky_mat := ProceduralSkyMaterial.new()
	sky_mat.sky_top_color = world.look_color("sky_top", Color(0.30, 0.44, 0.64) if winter else Color(0.22, 0.44, 0.78))
	sky_mat.sky_horizon_color = world.look_color("sky_horizon", Color(0.80, 0.84, 0.88) if winter else Color(0.66, 0.78, 0.90))
	sky_mat.ground_horizon_color = sky_mat.sky_horizon_color
	sky_mat.ground_bottom_color = sky_mat.sky_horizon_color
	sky_mat.sun_angle_max = 6.0
	var sky := Sky.new()
	sky.sky_material = sky_mat
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_sky_contribution = 0.45
	env.ambient_light_color = Color(0.86, 0.86, 0.80)
	env.ambient_light_energy = float(look.get("ambient", 1.0))
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.tonemap_white = 5.0
	env.ssao_enabled = true
	env.ssao_radius = 2.2
	env.ssao_intensity = 1.1
	env.fog_enabled = true
	env.fog_light_color = world.look_color("fog_color", Color(0.78, 0.84, 0.92) if winter else Color(0.70, 0.80, 0.93))
	env.fog_density = 0.0009 * haze
	env.fog_aerial_perspective = 0.7
	env.fog_sky_affect = 0.0
	env.fog_sun_scatter = 0.12
	env.glow_enabled = true
	env.glow_intensity = 0.35
	env.glow_bloom = 0.04
	var we := WorldEnvironment.new()
	we.environment = env
	stage.add_child(we)

	var az := deg_to_rad(float(look.get("sun_azimuth", 215.0)))
	var el := deg_to_rad(float(look.get("sun_elevation", 38.0)))
	var sun_dir := WorldData.g3(sin(az) * cos(el), cos(az) * cos(el), sin(el))
	var sun := DirectionalLight3D.new()
	sun.light_color = world.look_color("sun_color", Color(1.0, 0.93, 0.84) if winter else Color(1.0, 0.95, 0.86))
	sun.light_energy = float(look.get("sun_energy", 1.25 if winter else 1.5))
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = SHADOW_REACH_SHOTS_M if shots else SHADOW_REACH_WALK_M
	sun.directional_shadow_blend_splits = true
	sun.shadow_bias = 0.06
	sun.shadow_normal_bias = 1.6
	stage.add_child(sun)
	var mid := world.centre(SUN_AIM_HEIGHT_M)
	sun.look_at_from_position(mid + sun_dir * SUN_DISTANCE_M, mid, Vector3.UP)
