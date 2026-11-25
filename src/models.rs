//! Textured 3D models generated with fal.ai (Tripo): Bila, the passers-by and the motorbike.
//! They come as static meshes, so walking, riding and arm swing are done in the vertex shader
//! by bending the legs and arms around the hips and shoulders.

use eframe::glow::{self, HasContext};
use glam::{Mat4, Vec3};

pub const M_BILA: usize = 0;
pub const M_FEMME_WAX: usize = 1;
pub const M_HOMME_BOUBOU: usize = 2;
pub const M_JEUNE: usize = 3;
pub const M_FEMME_VILLE: usize = 4;
pub const M_MOTO: usize = 5;
pub const PEOPLE: [usize; 4] = [M_FEMME_WAX, M_HOMME_BOUBOU, M_JEUNE, M_FEMME_VILLE];

struct Src {
    bytes: &'static [u8],
    /// metres: height for people, length for the moto
    size: f32,
    /// rotation (about Y) that turns the model to face +X
    yaw: f32,
    person: bool,
}

fn sources() -> [Src; 6] {
    [
        Src { bytes: include_bytes!("../assets/models/bila.glb"), size: 1.80, yaw: 0.0, person: true },
        Src { bytes: include_bytes!("../assets/models/femme_wax.glb"), size: 1.68, yaw: 0.0, person: true },
        Src { bytes: include_bytes!("../assets/models/homme_boubou.glb"), size: 1.76, yaw: 0.0, person: true },
        Src { bytes: include_bytes!("../assets/models/jeune_homme.glb"), size: 1.76, yaw: 0.0, person: true },
        Src { bytes: include_bytes!("../assets/models/femme_ville.glb"), size: 1.68, yaw: 0.0, person: true },
        Src { bytes: include_bytes!("../assets/models/moto.glb"), size: 2.0, yaw: -std::f32::consts::FRAC_PI_2, person: false },
    ]
}

/// One instance to draw this frame.
#[derive(Clone, Copy)]
pub struct Inst {
    pub model: usize,
    pub mat: Mat4,
    pub phase: f32,
    pub moving: f32,
    pub seated: bool,
    pub tint: [f32; 3],
    pub aim: bool,
}

struct Gpu {
    vao: glow::VertexArray,
    _vbo: glow::Buffer,
    count: i32,
    base: glow::Texture,
    normal: glow::Texture,
    person: bool,
}

pub struct Models {
    list: Vec<Gpu>,
    prog: glow::Program,
}

/// Vertex data in model space: feet (or wheels) at y = 0, height 1 for people, facing +X.
struct Cpu {
    verts: Vec<f32>,
    base: (u32, u32, Vec<u8>),
    normal: (u32, u32, Vec<u8>),
    scale: f32,
}

fn to_rgba(img: &gltf::image::Data) -> Vec<u8> {
    use gltf::image::Format;
    match img.format {
        Format::R8G8B8A8 => img.pixels.clone(),
        Format::R8G8B8 => img.pixels.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        Format::R8 => img.pixels.iter().flat_map(|&c| [c, c, c, 255]).collect(),
        Format::R8G8 => img.pixels.chunks(2).flat_map(|c| [c[0], c[1], 0, 255]).collect(),
        _ => vec![200; (img.width * img.height * 4) as usize],
    }
}

fn shrink(w: u32, h: u32, px: Vec<u8>, max: u32) -> (u32, u32, Vec<u8>) {
    if w <= max && h <= max {
        return (w, h, px);
    }
    let Some(img) = image::RgbaImage::from_raw(w, h, px) else { return (1, 1, vec![200, 200, 200, 255]) };
    let (nw, nh) = (max.min(w), max.min(h));
    let r = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
    (nw, nh, r.into_raw())
}

fn load(src: &Src) -> Result<Cpu, String> {
    let (doc, buffers, images) = gltf::import_slice(src.bytes).map_err(|e| e.to_string())?;
    let mut pos: Vec<Vec3> = Vec::new();
    let mut nrm: Vec<Vec3> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut base = (1, 1, vec![200u8, 200, 200, 255]);
    let mut normal = (1, 1, vec![128u8, 128, 255, 255]);
    for mesh in doc.meshes() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| Some(&buffers[b.index()]));
            let p: Vec<[f32; 3]> = reader.read_positions().map(|i| i.collect()).unwrap_or_default();
            let n: Vec<[f32; 3]> = reader.read_normals().map(|i| i.collect()).unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; p.len()]);
            let t: Vec<[f32; 2]> = reader.read_tex_coords(0).map(|i| i.into_f32().collect()).unwrap_or_else(|| vec![[0.0, 0.0]; p.len()]);
            let idx: Vec<u32> = reader.read_indices().map(|i| i.into_u32().collect()).unwrap_or_else(|| (0..p.len() as u32).collect());
            for i in idx {
                let i = i as usize;
                pos.push(Vec3::from(p[i]));
                nrm.push(Vec3::from(n[i]));
                uv.push(t[i]);
            }
            let mat = prim.material();
            if let Some(t) = mat.pbr_metallic_roughness().base_color_texture() {
                let img = &images[t.texture().source().index()];
                base = shrink(img.width, img.height, to_rgba(img), 1024);
            }
            if let Some(t) = mat.normal_texture() {
                let img = &images[t.texture().source().index()];
                normal = shrink(img.width, img.height, to_rgba(img), 1024);
            }
        }
    }
    if pos.is_empty() {
        return Err("modèle vide".into());
    }
    // turn to face +X, feet on the ground, centred
    let rot = Mat4::from_rotation_y(src.yaw);
    for (p, n) in pos.iter_mut().zip(nrm.iter_mut()) {
        *p = rot.transform_point3(*p);
        *n = rot.transform_vector3(*n);
    }
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for p in &pos {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let ext = hi - lo;
    let norm = if src.person { ext.y } else { ext.x.max(ext.z) };
    let centre = Vec3::new((lo.x + hi.x) * 0.5, lo.y, (lo.z + hi.z) * 0.5);
    let mut verts = Vec::with_capacity(pos.len() * 8);
    for i in 0..pos.len() {
        let p = (pos[i] - centre) / norm;
        let n = nrm[i].normalize_or_zero();
        verts.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, uv[i][0], uv[i][1]]);
    }
    Ok(Cpu { verts, base, normal, scale: src.size })
}

/// Load every model once (CPU side); call `upload` on the GL thread.
pub fn load_all() -> Vec<Result<(Vec<f32>, (u32, u32, Vec<u8>), (u32, u32, Vec<u8>), f32, bool), String>> {
    sources()
        .iter()
        .map(|s| load(s).map(|c| (c.verts, c.base, c.normal, c.scale, s.person)))
        .collect()
}

/// Model scale in metres (height of a person, length of the moto).
pub fn scale_of(model: usize) -> f32 {
    [1.80, 1.68, 1.76, 1.76, 1.68, 2.0][model.min(5)]
}

const VS: &str = r#"
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_nrm;
layout(location=2) in vec2 a_uv;
uniform mat4 u_vp; uniform mat4 u_model; uniform mat4 u_lvp;
uniform float u_phase; uniform float u_move; uniform float u_seated; uniform float u_person; uniform float u_aim;
out vec3 v_pos; out vec3 v_nrm; out vec2 v_uv; out vec4 v_sc;
mat3 rotz(float a) { float c = cos(a), s = sin(a); return mat3(c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0); }
mat3 rotx(float a) { float c = cos(a), s = sin(a); return mat3(1.0, 0.0, 0.0, 0.0, c, s, 0.0, -s, c); }
void main() {
    vec3 p = a_pos; vec3 n = a_nrm;
    if (u_person > 0.5) {
        float side = p.z >= 0.0 ? 1.0 : -1.0;
        const float HIP = 0.52;
        const float SH_Y = 0.80;
        if (abs(p.z) > 0.105 && p.y > 0.42 && p.y < 0.86 && (p.y > 0.6 || abs(p.z) > 0.165)) {
            // arms: lower them from the A-pose, then swing
            vec3 piv = vec3(0.0, SH_Y, side * 0.09);
            float w = clamp((abs(p.z) - 0.105) / 0.04, 0.0, 1.0);
            bool aim_arm = u_aim > 0.5 && side > 0.0;
            mat3 r = rotz(aim_arm ? 1.45 : (u_seated > 0.5 ? 1.0 : -sin(u_phase) * 0.45 * u_move * side)) * rotx(side * (aim_arm ? 0.62 : 0.55) * w);
            p = piv + r * (p - piv);
            n = r * n;
        } else if (p.y < HIP && abs(p.z) < 0.2) {
            float w = clamp((HIP - p.y) / 0.08, 0.0, 1.0);
            float a = (u_seated > 0.5 ? 1.35 : sin(u_phase) * 0.5 * u_move * side) * w;
            vec3 piv = vec3(0.0, HIP, p.z);
            mat3 r = rotz(a);
            p = piv + r * (p - piv);
            n = r * n;
        }
    }
    vec4 wp = u_model * vec4(p, 1.0);
    v_pos = wp.xyz;
    v_nrm = normalize(mat3(u_model) * n);
    v_uv = a_uv;
    v_sc = u_lvp * vec4(wp.xyz + v_nrm * 0.03, 1.0);
    gl_Position = u_vp * wp;
}
"#;

const FS: &str = r#"
precision highp float;
precision highp sampler2DShadow;
uniform sampler2D u_base; uniform sampler2D u_nmap; uniform sampler2DShadow u_shadow;
uniform vec3 u_cam; uniform vec3 u_sun; uniform vec3 u_sun_col; uniform vec3 u_amb; uniform vec3 u_fogc; uniform vec3 u_tint;
uniform float u_fog; uniform float u_texel; uniform float u_depth_only;
in vec3 v_pos; in vec3 v_nrm; in vec2 v_uv; in vec4 v_sc;
out vec4 frag;
float shadowing() {
    vec3 sc = v_sc.xyz / v_sc.w * 0.5 + 0.5;
    if (sc.x < 0.0 || sc.x > 1.0 || sc.y < 0.0 || sc.y > 1.0 || sc.z > 1.0) return 1.0;
    float s = 0.0;
    for (int x = -1; x <= 1; x++)
        for (int y = -1; y <= 1; y++)
            s += texture(u_shadow, vec3(sc.xy + vec2(float(x), float(y)) * u_texel, sc.z - 0.0012));
    return s / 9.0;
}
void main() {
    if (u_depth_only > 0.5) { frag = vec4(1.0); return; }
    vec4 t = texture(u_base, v_uv);
    vec3 N = normalize(v_nrm);
    if (!gl_FrontFacing) N = -N;
    vec3 nm = texture(u_nmap, v_uv).xyz * 2.0 - 1.0;
    vec3 dp1 = dFdx(v_pos); vec3 dp2 = dFdy(v_pos);
    vec2 du1 = dFdx(v_uv); vec2 du2 = dFdy(v_uv);
    vec3 dp2perp = cross(dp2, N); vec3 dp1perp = cross(N, dp1);
    vec3 T = dp2perp * du1.x + dp1perp * du2.x;
    vec3 B = dp2perp * du1.y + dp1perp * du2.y;
    float m = max(dot(T, T), dot(B, B));
    vec3 n = m > 1e-12 ? normalize(mat3(T * inversesqrt(m), B * inversesqrt(m), N) * nm) : N;
    float diff = max(dot(n, u_sun), 0.0);
    float sh = shadowing();
    vec3 albedo = t.rgb * u_tint;
    vec3 col = albedo * (u_amb * (0.85 + 0.25 * n.y) + u_sun_col * diff * sh);
    vec3 v = normalize(u_cam - v_pos);
    float rim = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    col += u_amb * rim * 0.25;
    col += u_sun_col * 0.08 * pow(max(dot(n, normalize(v + u_sun)), 0.0), 24.0) * sh;
    float d = length(v_pos - u_cam);
    float f = clamp((d - u_fog * 0.25) / (u_fog * 0.75), 0.0, 1.0);
    frag = vec4(mix(col, u_fogc, f * f * (3.0 - 2.0 * f)), 1.0);
}
"#;

unsafe fn texture(gl: &glow::Context, (w, h, px): &(u32, u32, Vec<u8>)) -> Result<glow::Texture, String> {
    let t = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(t));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, *w as i32, *h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(px)));
    gl.generate_mipmap(glow::TEXTURE_2D);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(t)
}

pub struct Frame<'a> {
    pub vp: &'a Mat4,
    pub lvp: &'a Mat4,
    pub cam: Vec3,
    pub sun: Vec3,
    pub sun_col: Vec3,
    pub ambient: Vec3,
    pub fog_col: Vec3,
    pub fog: f32,
    pub texel: f32,
}

impl Models {
    pub fn new(gl: &glow::Context, compile: impl Fn(&str, &str) -> Result<glow::Program, String>) -> Result<Models, String> {
        let prog = compile(VS, FS)?;
        let mut list = Vec::new();
        for m in load_all() {
            let (verts, base, normal, _scale, person) = m?;
            unsafe {
                let vao = gl.create_vertex_array()?;
                let vbo = gl.create_buffer()?;
                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                let bytes = std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * 4);
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
                for (loc, size, off) in [(0u32, 3i32, 0i32), (1, 3, 3), (2, 2, 6)] {
                    gl.enable_vertex_attrib_array(loc);
                    gl.vertex_attrib_pointer_f32(loc, size, glow::FLOAT, false, 32, off * 4);
                }
                gl.bind_vertex_array(None);
                list.push(Gpu { vao, _vbo: vbo, count: (verts.len() / 8) as i32, base: texture(gl, &base)?, normal: texture(gl, &normal)?, person });
            }
        }
        Ok(Models { list, prog })
    }

    /// Draw every instance; `depth_only` for the shadow pass (vp = light matrix there).
    pub fn draw(&self, gl: &glow::Context, f: &Frame, insts: &[Inst], depth_only: bool, shadow_tex: glow::Texture) {
        unsafe {
            gl.use_program(Some(self.prog));
            let l = |n: &str| gl.get_uniform_location(self.prog, n);
            gl.uniform_matrix_4_f32_slice(l("u_vp").as_ref(), false, &f.vp.to_cols_array());
            gl.uniform_matrix_4_f32_slice(l("u_lvp").as_ref(), false, &f.lvp.to_cols_array());
            gl.uniform_3_f32(l("u_cam").as_ref(), f.cam.x, f.cam.y, f.cam.z);
            gl.uniform_3_f32(l("u_sun").as_ref(), f.sun.x, f.sun.y, f.sun.z);
            gl.uniform_3_f32(l("u_sun_col").as_ref(), f.sun_col.x, f.sun_col.y, f.sun_col.z);
            gl.uniform_3_f32(l("u_amb").as_ref(), f.ambient.x, f.ambient.y, f.ambient.z);
            gl.uniform_3_f32(l("u_fogc").as_ref(), f.fog_col.x, f.fog_col.y, f.fog_col.z);
            gl.uniform_1_f32(l("u_fog").as_ref(), f.fog);
            gl.uniform_1_f32(l("u_texel").as_ref(), f.texel);
            gl.uniform_1_f32(l("u_depth_only").as_ref(), if depth_only { 1.0 } else { 0.0 });
            gl.uniform_1_i32(l("u_base").as_ref(), 4);
            gl.uniform_1_i32(l("u_nmap").as_ref(), 5);
            gl.uniform_1_i32(l("u_shadow").as_ref(), 6);
            if !depth_only {
                gl.active_texture(glow::TEXTURE6);
                gl.bind_texture(glow::TEXTURE_2D, Some(shadow_tex));
            }
            let (lm, lphase, lmove, lseat, lperson, ltint, laim) = (l("u_model"), l("u_phase"), l("u_move"), l("u_seated"), l("u_person"), l("u_tint"), l("u_aim"));
            for i in insts {
                let Some(m) = self.list.get(i.model) else { continue };
                gl.active_texture(glow::TEXTURE4);
                gl.bind_texture(glow::TEXTURE_2D, Some(m.base));
                gl.active_texture(glow::TEXTURE5);
                gl.bind_texture(glow::TEXTURE_2D, Some(m.normal));
                let mat = i.mat * Mat4::from_scale(Vec3::splat(scale_of(i.model)));
                gl.uniform_matrix_4_f32_slice(lm.as_ref(), false, &mat.to_cols_array());
                gl.uniform_1_f32(lphase.as_ref(), i.phase);
                gl.uniform_1_f32(lmove.as_ref(), i.moving);
                gl.uniform_1_f32(lseat.as_ref(), if i.seated { 1.0 } else { 0.0 });
                gl.uniform_1_f32(lperson.as_ref(), if m.person { 1.0 } else { 0.0 });
                gl.uniform_3_f32(ltint.as_ref(), i.tint[0], i.tint[1], i.tint[2]);
                gl.uniform_1_f32(laim.as_ref(), if i.aim { 1.0 } else { 0.0 });
                gl.bind_vertex_array(Some(m.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, m.count);
            }
            gl.bind_vertex_array(None);
            for u in [glow::TEXTURE6, glow::TEXTURE5, glow::TEXTURE4] {
                gl.active_texture(u);
                gl.bind_texture(glow::TEXTURE_2D, None);
            }
            gl.active_texture(glow::TEXTURE0);
        }
    }
}
