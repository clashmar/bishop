//! OLD MACROQUAD IMPL DO NOT UPDATE THIS FILE
// // engine_core/src/lighting/render_system.rs
// use crate::shaders::shaders::*;
// use crate::prelude::*;
// use bishop::prelude::*;

// /// Max lights per layer.
// pub const MAX_LIGHTS: usize = 10;

// #[derive(Clone, Copy, Default)]
// pub struct LightBuffer {
//     pos: Vec2,
//     color: Vec3,
//     intensity: f32,
//     radius: f32,
//     spread: f32,
//     alpha: f32,
//     brightness: f32,
// }

// #[derive(Clone, Copy, Default)]
// pub struct GlowBuffer {
//     brightness: f32,
//     color: Vec3,
//     intensity: f32,
//     pos: Vec2,
//     emission: f32,
//     mask_size: Vec2,
// }

// pub struct RenderSystem {
//     // Render targets
//     pub scene_rt: RenderTarget,
//     pub ambient_rt: RenderTarget,
//     pub glow_rt: RenderTarget,
//     pub undarkened_rt: RenderTarget,
//     pub spot_rt: RenderTarget,
//     pub mask_rt: RenderTarget,
//     pub scene_comp_rt: RenderTarget,
//     pub final_comp_rt: RenderTarget,
//     /// Materials
//     pub ambient_mat: Material,
//     pub glow_mat: Material,
//     pub undarkened_mat: Material,
//     pub spot_mat: Material,
//     pub scene_comp_mat: Material,
//     pub final_comp_mat: Material,
//     /// Cached light data
//     light_bufffers: [LightBuffer; MAX_LIGHTS],
//     glow_bufffers: [GlowBuffer; MAX_LIGHTS],
//     /// Time spent rendering last frame (ms)
//     pub render_time_ms: f32,
//     /// Current render target dimensions
//     rt_width: f32,
//     rt_height: f32,
// }

// impl RenderSystem {
//     /// Create a new render system with render targets sized for the given grid size.
//     pub fn with_grid_size(grid_size: f32) -> Self {
//         let width = world_virtual_width(grid_size) as u32;
//         let height = world_virtual_height(grid_size) as u32;

//         let make_render_target = || {
//             let rt = render_target(width, height);
//             rt.texture.set_filter(FilterMode::Nearest);
//             rt
//         };

//         let glow_mat = load_material(
//             ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: GLOW_FRAGMENT_SHADER },
//             MaterialParams {
//                 uniforms: vec![
//                     UniformDesc::new("GlowCount", UniformType::Int1),
//                     UniformDesc::new("Brightness", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("Intensity", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("Color", UniformType::Float3).array(MAX_LIGHTS),
//                     UniformDesc::new("Emission", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("maskPos", UniformType::Float2).array(MAX_LIGHTS),
//                     UniformDesc::new("maskSize", UniformType::Float2).array(MAX_LIGHTS),
//                     UniformDesc::new("screenWidth", UniformType::Float1),
//                     UniformDesc::new("screenHeight", UniformType::Float1),
//                 ],
//                 textures: vec![
//                     "scene_tex".to_string(),
//                     "tex_mask0".to_string(),
//                     "tex_mask1".to_string(),
//                     "tex_mask2".to_string(),
//                     "tex_mask3".to_string(),
//                     "tex_mask4".to_string(),
//                     "tex_mask5".to_string(),
//                     "tex_mask6".to_string(),
//                     "tex_mask7".to_string(),
//                     "tex_mask8".to_string(),
//                 ],
//                 ..Default::default()
//             },
//         ).unwrap();

//         let ambient_mat = load_material(
//             ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: AMB_FRAGMENT_SHADER },
//             MaterialParams {
//                 uniforms: vec![UniformDesc::new("Darkness", UniformType::Float1)],
//                 textures: vec!["tex".to_string()],
//                 ..Default::default()
//             },
//         ).unwrap();

//         let undarkened_mat = load_material(
//             ShaderSource::Glsl {
//                 vertex: VERTEX_SHADER,
//                 fragment: UNDARKENED_FRAGMENT_SHADER,
//             },
//             MaterialParams {
//                 textures: vec![
//                     "scene_tex".to_string(),
//                     "glow_tex".to_string(),
//                     "undarkened_tex".to_string(),
//                 ],
//                 ..Default::default()
//             },
//         )
//         .unwrap();

//         let spot_mat = load_material(
//             ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: SPOT_FRAGMENT_SHADER },
//             MaterialParams {
//                 uniforms: vec![
//                     UniformDesc::new("LightCount", UniformType::Int1),
//                     UniformDesc::new("LightPos", UniformType::Float2).array(MAX_LIGHTS),
//                     UniformDesc::new("LightColor", UniformType::Float3).array(MAX_LIGHTS),
//                     UniformDesc::new("LightIntensity", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("LightRadius", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("LightSpread", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("LightAlpha", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("LightBrightness", UniformType::Float1).array(MAX_LIGHTS),
//                     UniformDesc::new("ScreenWidth", UniformType::Float1),
//                     UniformDesc::new("ScreenHeight", UniformType::Float1),
//                     UniformDesc::new("Darkness", UniformType::Float1),
//                 ],
//                 textures: vec!["tex".to_string(), "light_mask".to_string()],
//                 ..Default::default()
//             },
//         ).unwrap();

//         let scene_comp_mat = load_material(
//             ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: SCENE_FRAGMENT_SHADER },
//             MaterialParams {
//                 pipeline_params: PipelineParams {
//                     color_blend: Some(BlendState::new(
//                         Equation::Add,
//                         BlendFactor::One,
//                         BlendFactor::OneMinusValue(BlendValue::SourceAlpha)
//                     )),
//                     alpha_blend: Some(BlendState::new(
//                         Equation::Add,
//                         BlendFactor::Value(BlendValue::SourceAlpha),
//                         BlendFactor::OneMinusValue(BlendValue::SourceAlpha)
//                     )),
//                     ..Default::default()
//                 },
//                 textures: vec![
//                     "amb_tex".to_string(),
//                     "glow_tex".to_string(),
//                     "scene_comp_tex".to_string(),
//                 ],
//                 ..Default::default()
//             },
//         ).unwrap();

//         let final_comp_mat = load_material(
//             ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: COMPOSITE_FRAGMENT_SHADER },
//             MaterialParams {
//                 textures: vec![
//                     "scene_comp_tex".to_string(),
//                     "spot_tex".to_string(),
//                     "final_comp_tex".to_string(),
//                 ],
//                 ..Default::default()
//             },
//         ).unwrap();

//         Self {
//             scene_rt: make_render_target(),
//             ambient_rt: make_render_target(),
//             glow_rt: make_render_target(),
//             undarkened_rt: make_render_target(),
//             spot_rt: make_render_target(),
//             mask_rt: make_render_target(),
//             scene_comp_rt: make_render_target(),
//             final_comp_rt: make_render_target(),
//             ambient_mat,
//             glow_mat,
//             undarkened_mat,
//             spot_mat,
//             scene_comp_mat,
//             final_comp_mat,
//             light_bufffers: [LightBuffer::default(); MAX_LIGHTS],
//             glow_bufffers: [GlowBuffer::default(); MAX_LIGHTS],
//             render_time_ms: 0.0,
//             rt_width: width as f32,
//             rt_height: height as f32,
//         }
//     }

//     /// Create a new render system with default grid size (16.0).
//     pub fn new() -> Self {
//         Self::with_grid_size(crate::constants::DEFAULT_GRID_SIZE)
//     }

//     /// Applies darkness to the scene.
//     pub fn run_ambient_pass(
//         &mut self,
//         darkness: f32,
//     ) {
//         self.clear_cam(&self.ambient_rt);

//         self.ambient_mat.set_texture("tex", self.scene_rt.texture.clone());
//         self.ambient_mat.set_uniform("Darkness", darkness);

//         self.draw_pass(&self.ambient_mat, &self.scene_rt.texture);
//     }

//     /// Renders glow textures per-layer in the room.
//     pub fn run_glow_pass(
//         &mut self,
//         render_cam: &Camera2D,
//         glows: Vec<(&Glow, Vec2)>,
//         asset_manager: &mut AssetManager,
//     ) {
//         self.clear_cam(&self.glow_rt);
//         self.clear_glow_buffers();
//         if glows.is_empty() {
//             return;
//         }

//         let preview = render_cam.render_target.is_some();
//         let (target_w, target_h) = if preview {
//             (self.rt_width, self.rt_height)
//         } else {
//             (screen_width(), screen_height())
//         };

//         self.glow_mat.set_texture("scene_tex", self.scene_rt.texture.clone());

//         for (i, (glow, world_pos)) in glows.iter().take(MAX_LIGHTS).enumerate() {
//             let tex = asset_manager.get_texture_from_id(glow.sprite_id).clone();
//             self.glow_mat.set_texture(&format!("tex_mask{}", i), tex);

//             let screen_pos = world_to_target(
//                 render_cam,
//                 *world_pos,
//                 target_w,
//                 target_h,
//                 preview
//             );

//             let buffer = &mut self.glow_bufffers[i];
//             buffer.pos = (screen_pos.x, screen_pos.y).into();
//             buffer.color = (glow.color.x, glow.color.y, glow.color.z).into();
//             buffer.intensity = glow.intensity;
//             buffer.brightness = glow.brightness;
//             buffer.emission = glow.emission;

//             // Texture dimensions
//             if let Some((w, h)) = asset_manager.texture_size(glow.sprite_id) {
//                 buffer.mask_size = Vec2::new(
//                     world_distance_to_uniform_target(render_cam, w, target_w),
//                     world_distance_to_uniform_target(render_cam, h, target_w),
//                 );
//             }
//         }

//         self.glow_mat.set_uniform("GlowCount", glows.len().min(MAX_LIGHTS) as i32);
//         self.glow_mat.set_uniform_array("Brightness", &self.glow_bufffers.map(|g| g.brightness));
//         self.glow_mat.set_uniform_array("Intensity", &self.glow_bufffers.map(|g| g.intensity));
//         self.glow_mat.set_uniform_array("Color", &self.glow_bufffers.map(|g| g.color));
//         self.glow_mat.set_uniform_array("Emission", &self.glow_bufffers.map(|g| g.emission));
//         self.glow_mat.set_uniform_array("maskPos", &self.glow_bufffers.map(|g| g.pos));
//         self.glow_mat.set_uniform_array("maskSize", &self.glow_bufffers.map(|g| g.mask_size));
//         self.glow_mat.set_uniform("screenWidth", target_w);
//         self.glow_mat.set_uniform("screenHeight", target_h);

//         self.draw_pass(&self.glow_mat, &self.glow_rt.texture);

//         gl_use_default_material();
//     }

//     /// Renders the scene without applying darkness so the lighting pass can operate
//     /// on an undimmed texture.
//     pub fn run_undarkened_pass(&mut self) {
//         let cam = Camera2D {
//             target: vec2(self.rt_width * 0.5, self.rt_height * 0.5),
//             zoom:   vec2(2.0 / self.rt_width, 2.0 / self.rt_height),
//             render_target: Some(self.undarkened_rt.clone()),
//             ..Default::default()
//         };
//         set_camera(&cam);

//         self.undarkened_mat.set_texture("scene_tex", self.scene_rt.texture.clone());
//         self.undarkened_mat.set_texture("glow_tex", self.glow_rt.texture.clone());
//         self.undarkened_mat.set_texture("undarkened_tex", self.undarkened_rt.texture.clone());

//         self.draw_pass(&self.undarkened_mat, &self.undarkened_rt.texture);
//     }

//     /// Renders spotlights using the undarkened scene texture.
//     pub fn run_spotlight_pass(
//         &mut self,
//         render_cam: &Camera2D,
//         lights: Vec<(Vec2, Light)>,
//         darkness: f32,
//     ) {
//         self.clear_cam(&self.spot_rt);
//         self.clear_light_buffers();

//         if !lights.is_empty() {
//             let preview = render_cam.render_target.is_some();
//             let (target_w, target_h) = if preview {
//                 (self.rt_width, self.rt_height)
//             } else {
//                 (screen_width(), screen_height())
//             };

//             let light_count = lights.len();

//             for i in 0..light_count {
//                 let (pos, l) = &lights[i];
//                 let world_pos = *pos + l.pos;
//                 let buffer = &mut self.light_bufffers[i];

//                 let target_vec = world_to_target(
//                     render_cam,
//                     world_pos,
//                     target_w,
//                     target_h,
//                     preview
//                 );

//                 buffer.pos = (target_vec.x, target_vec.y).into();

//                 buffer.radius = world_distance_to_uniform_target(render_cam, l.radius, target_w);
//                 buffer.spread = world_distance_to_uniform_target(render_cam, l.spread, target_w);
//                 buffer.color = (l.color.x, l.color.y, l.color.z).into();
//                 buffer.intensity = l.intensity;
//                 buffer.alpha = l.alpha;
//                 buffer.brightness = l.brightness;
//             }

//             self.spot_mat.set_texture("tex", self.undarkened_rt.texture.clone());
//             self.spot_mat.set_texture("light_mask", self.mask_rt.texture.clone());

//             self.spot_mat.set_uniform("LightCount", light_count as i32);
//             self.spot_mat.set_uniform_array("LightPos", &self.light_bufffers.map(|g| g.pos));
//             self.spot_mat.set_uniform_array("LightColor", &self.light_bufffers.map(|g| g.color));
//             self.spot_mat.set_uniform_array("LightIntensity", &self.light_bufffers.map(|g| g.intensity));
//             self.spot_mat.set_uniform_array("LightRadius", &self.light_bufffers.map(|g| g.radius));
//             self.spot_mat.set_uniform_array("LightSpread", &self.light_bufffers.map(|g| g.spread));
//             self.spot_mat.set_uniform_array("LightAlpha", &self.light_bufffers.map(|g| g.alpha));
//             self.spot_mat.set_uniform_array("LightBrightness", &self.light_bufffers.map(|g| g.brightness));
//             self.spot_mat.set_uniform("ScreenWidth", target_w);
//             self.spot_mat.set_uniform("ScreenHeight", target_h);
//             self.spot_mat.set_uniform("Darkness", darkness);

//             self.draw_pass(&self.spot_mat, &self.spot_rt.texture);
//         }
//     }

//     /// Composites the per-layer room textures.
//     pub fn run_scene_pass(&mut self) {
//         let scene_comp_cam = Camera2D {
//             target: Vec2::new(self.rt_width * 0.5, self.rt_height * 0.5),
//             zoom: Vec2::new(2.0 / self.rt_width, 2.0 / self.rt_height),
//             render_target: Some(self.scene_comp_rt.clone()),
//             ..Default::default()
//         };

//         set_camera(&scene_comp_cam);

//         self.scene_comp_mat.set_texture("amb_tex", self.ambient_rt.texture.clone());
//         self.scene_comp_mat.set_texture("glow_tex", self.glow_rt.texture.clone());
//         self.scene_comp_mat.set_texture("scene_comp_tex", self.scene_comp_rt.texture.clone());

//         self.draw_pass(&self.scene_comp_mat, &self.scene_comp_rt.texture);
//     }

//     /// The last composite stage for rendering a room before post-processing.
//     pub fn run_final_pass(&mut self) {
//         self.clear_cam(&self.final_comp_rt);

//         self.final_comp_mat.set_texture("scene_comp_tex", self.scene_comp_rt.texture.clone());
//         self.final_comp_mat.set_texture("spot_tex", self.spot_rt.texture.clone());
//         self.final_comp_mat.set_texture("final_comp_tex", self.final_comp_rt.texture.clone());

//         self.draw_pass(&self.final_comp_mat, &self.final_comp_rt.texture);
//     }

//     /// Presents the final visual of the game with hybrid scaling.
//     /// Uses fractional scale to fill screen while maintaining aspect ratio, minimizing letterboxing.
//     pub fn present_game(&self) {
//         set_default_camera();
//         let tex = &self.final_comp_rt.texture;

//         let virt_w = self.rt_width;
//         let virt_h = self.rt_height;
//         let win_w = screen_width();
//         let win_h = screen_height();

//         // Hybrid scaling: use fractional scale to fill screen while maintaining aspect ratio
//         let scale = (win_w / virt_w).min(win_h / virt_h);
//         let scaled_w = virt_w * scale;
//         let scaled_h = virt_h * scale;

//         let offset_x = ((win_w - scaled_w) / 2.0).floor();
//         let offset_y = ((win_h - scaled_h) / 2.0).floor();

//         draw_texture_ex(
//             tex,
//             offset_x,
//             offset_y,
//             Color::WHITE,
//             DrawTextureParams {
//                 dest_size: Some(Vec2::new(scaled_w, scaled_h)),
//                 ..Default::default()
//             },
//         );
//     }

//     /// Presents the render target directly at 1:1 for window-sized targets.
//     pub fn present(&self) {
//         set_default_camera();
//         draw_texture_ex(
//             &self.final_comp_rt.texture,
//             0.0,
//             0.0,
//             Color::WHITE,
//             DrawTextureParams::default(),
//         );
//     }

//     /// Sets and draws the supplied material and resets to default.
//     pub fn draw_pass(&self, material: &Material, quad: &Texture2D) {
//         gl_use_material(material);
//         draw_texture_ex(
//             quad,
//             0.0,
//             0.0,
//             Color::WHITE,
//             DrawTextureParams::default(),
//         );
//         gl_use_default_material();
//     }

//     /// Sets the mask render target background to white.
//     pub fn init_mask_cam(&self) {
//         let mask_cam = Camera2D {
//             target: Vec2::new(self.rt_width * 0.5, self.rt_height * 0.5),
//             zoom: Vec2::new(2.0 / self.rt_width, 2.0 / self.rt_height),
//             render_target: Some(self.mask_rt.clone()),
//             ..Default::default()
//         };
//         set_camera(&mask_cam);
//         clear_background(Color::WHITE);
//         gl_use_default_material();
//     }

//     /// Sets, clears and returns the scene camera.
//     pub fn clear_scene_cam(&self, render_cam: &Camera2D) -> Camera2D {
//         let scene_cam = Camera2D {
//             target: render_cam.target,
//             zoom: render_cam.zoom,
//             render_target: Some(self.scene_rt.clone()),
//             ..Default::default()
//         };

//         // Clear the texture every layer
//         set_camera(&scene_cam); // Set before clearing!
//         clear_background(Color::TRANSPARENT);
//         scene_cam
//     }

//     /// Sets, clears the given render target and returns the camera for it.
//     pub fn clear_cam(&self, rt: &RenderTarget) -> Camera2D {
//         let cam = Camera2D {
//             target: vec2(self.rt_width * 0.5, self.rt_height * 0.5),
//             zoom: vec2(2.0 / self.rt_width, 2.0 / self.rt_height),
//             render_target: Some(rt.clone()),
//             ..Default::default()
//         };
//         set_camera(&cam);
//         clear_background(Color::TRANSPARENT);
//         cam
//     }

//     /// Reset the per‑frame light buffers.
//     pub fn clear_light_buffers(&mut self) {
//         self.light_bufffers
//             .iter_mut()
//             .for_each(|slot| *slot = LightBuffer::default());
//     }

//     /// Reset the per‑frame glow buffers.
//     pub fn clear_glow_buffers(&mut self) {
//         self.glow_bufffers
//             .iter_mut()
//             .for_each(|slot| *slot = GlowBuffer::default());
//     }

//     /// Resizes render targets to match the camera zoom.
//     pub fn resize_for_camera(&mut self, zoom: Vec2) {
//         let required_width = (2.0 / zoom.x).round() as u32;
//         let required_height = (2.0 / zoom.y).round() as u32;

//         if required_width != self.rt_width as u32 || required_height != self.rt_height as u32 {
//             self.resize(required_width, required_height);
//         }
//     }

//     /// Resizes render targets to match window size if they don't already.
//     pub fn resize_to_window(&mut self) {
//         let win_w = screen_width() as u32;
//         let win_h = screen_height() as u32;

//         if win_w != self.rt_width as u32 || win_h != self.rt_height as u32 {
//             self.resize(win_w, win_h);
//         }
//     }

//     /// Re-creates every render target with the supplied size.
//     pub fn resize(&mut self, width: u32, height: u32) {
//         self.rt_width = width as f32;
//         self.rt_height = height as f32;

//         let make = || {
//             let rt = render_target(width, height);
//             rt.texture.set_filter(FilterMode::Nearest);
//             rt
//         };

//         self.scene_rt = make();
//         self.ambient_rt = make();
//         self.glow_rt = make();
//         self.undarkened_rt = make();
//         self.spot_rt = make();
//         self.mask_rt = make();
//         self.scene_comp_rt = make();
//         self.final_comp_rt = make();

//         // Reset the mask cam
//         self.init_mask_cam();
//     }
// }

// /// Distance conversion for shader uniforms.
// fn world_distance_to_uniform_target(cam: &Camera2D, distance: f32, target_w: f32) -> f32 {
//     let scale = cam.zoom.x * target_w * 0.5;
//     (distance * scale).abs()
// }

// fn world_to_target(
//     cam: &Camera2D,
//     world_pos: Vec2,
//     target_w: f32,
//     target_h: f32,
//     preview: bool,
// ) -> Vec2 {
//     let screen = cam.world_to_screen(world_pos);
//     let scale_x = target_w / screen_width();
//     let scale_y = target_h / screen_height();

//     let x = screen.x * scale_x;
//     let mut y = screen.y * scale_y;
//     if preview {
//         y = target_h - y;
//     }

//     Vec2::new(x, y)
// }
