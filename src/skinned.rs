//! Characters rigged and animated in Blender (blender/rig_character.py): GPU skinning.
//! Files: assets/chars/<name>.ocm (mesh + weights + baked animations), <name>_c.jpg / _n.jpg.

use eframe::glow::{self, HasContext};
use glam::{Mat4, Vec3, Vec4};
use std::sync::OnceLock;

pub const C_BILA: usize = 0;
pub const C_FEMME_WAX: usize = 1;
pub const C_HOMME_BOUBOU: usize = 2;
pub const C_JEUNE: usize = 3;
pub const C_FEMME_VILLE: usize = 4;
pub const PEOPLE: [usize; 4] = [C_FEMME_WAX, C_HOMME_BOUBOU, C_JEUNE, C_FEMME_VILLE];
/// Residents built in Blender with MPFB (MakeHuman): name, woman?, age group (0 young, 1 adult, 2 elder)
pub const MPFB: [(&str, bool, u8); 10] = [
    ("h_jeune", false, 0),
    ("h_ouvrier", false, 1),
    ("h_cadre", false, 1),
    ("h_ancien", false, 2),
    ("h_sportif", false, 0),
    ("f_etudiante", true, 0),
    ("f_cadre", true, 1),
    ("f_sportive", true, 0),
    ("f_maman", true, 1),
    ("f_grandmere", true, 2),
];
pub const C_MPFB0: usize = 5;
/// Bila's outfits (wardrobe): t-shirt and jeans, dark suit, jacket
pub const BILA_OUTFITS: [(usize, &str); 3] = [(C_BILA, "T-shirt et jean"), (15, "Costume sombre"), (16, "Veste et jean")];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anim {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Aim = 3,
    Ride = 4,
    Drive = 5,
    Sit = 6,
    Eat = 7,
    Squat = 8,
    Punch = 9,
    Talk = 10,
}

pub struct Clip {
    pub frames: usize,
    pub fps: f32,
    /// frames * bones skin matrices
    pub mats: Vec<Mat4>,
}

pub struct CharData {
    pub verts: Vec<f32>,
    pub bones: Vec<String>,
    pub clips: Vec<Clip>,
    /// rest-pose centre of the vertices each bone moves most
    pub centers: Vec<Vec3>,
    pub color: &'static [u8],
    pub normal: &'static [u8],
}

impl CharData {
    pub fn bone(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b == name)
    }
    /// Skin matrices of every bone at time t (looping), blended between frames.
    pub fn sample(&self, anim: Anim, t: f32) -> Vec<Mat4> {
        let nb = self.bones.len();
        let Some(c) = self.clips.get(anim as usize) else { return vec![Mat4::IDENTITY; nb] };
        if c.frames == 0 {
            return vec![Mat4::IDENTITY; nb];
        }
        let f = (t * c.fps).max(0.0);
        let looped = anim != Anim::Punch;
        let (i0, i1, k) = if looped {
            let fi = f % c.frames as f32;
            let i0 = fi.floor() as usize % c.frames;
            (i0, (i0 + 1) % c.frames, fi.fract())
        } else {
            let fi = f.min((c.frames - 1) as f32);
            let i0 = fi.floor() as usize;
            (i0, (i0 + 1).min(c.frames - 1), fi.fract())
        };
        (0..nb).map(|b| {
            let a = c.mats[i0 * nb + b];
            let m = c.mats[i1 * nb + b];
            Mat4::from_cols(a.x_axis.lerp(m.x_axis, k), a.y_axis.lerp(m.y_axis, k), a.z_axis.lerp(m.z_axis, k), a.w_axis.lerp(m.w_axis, k))
        }).collect()
    }
}

fn parse(bytes: &[u8], color: &'static [u8], normal: &'static [u8]) -> Option<CharData> {
    let mut o = 4usize;
    if &bytes[0..4] != b"OCM1" {
        return None;
    }
    let u32_at = |o: &mut usize| -> u32 {
        let v = u32::from_le_bytes(bytes[*o..*o + 4].try_into().unwrap());
        *o += 4;
        v
    };
    let f32_at = |o: &mut usize| -> f32 {
        let v = f32::from_le_bytes(bytes[*o..*o + 4].try_into().unwrap());
        *o += 4;
        v
    };
    let nv = u32_at(&mut o) as usize;
    let nb = u32_at(&mut o) as usize;
    let na = u32_at(&mut o) as usize;
    let mut verts = Vec::with_capacity(nv * 16);
    for _ in 0..nv * 16 {
        verts.push(f32_at(&mut o));
    }
    let mut bones = Vec::new();
    for _ in 0..nb {
        let l = u32_at(&mut o) as usize;
        bones.push(String::from_utf8_lossy(&bytes[o..o + l]).to_string());
        o += l;
    }
    let mut clips = Vec::new();
    for _ in 0..na {
        let l = u32_at(&mut o) as usize;
        o += l;
        let frames = u32_at(&mut o) as usize;
        let fps = f32_at(&mut o);
        let mut mats = Vec::with_capacity(frames * nb);
        for _ in 0..frames * nb {
            let mut r = [[0f32; 4]; 3];
            for row in r.iter_mut() {
                for v in row.iter_mut() {
                    *v = f32_at(&mut o);
                }
            }
            // rows -> column-major
            mats.push(Mat4::from_cols(
                Vec4::new(r[0][0], r[1][0], r[2][0], 0.0),
                Vec4::new(r[0][1], r[1][1], r[2][1], 0.0),
                Vec4::new(r[0][2], r[1][2], r[2][2], 0.0),
                Vec4::new(r[0][3], r[1][3], r[2][3], 1.0),
            ));
        }
        clips.push(Clip { frames, fps, mats });
    }
    let mut acc = vec![(Vec3::ZERO, 0.0f32); nb];
    for v in verts.chunks(16) {
        let (j, w) = (v[8] as usize, v[12]);
        if j < nb && w > 0.6 {
            acc[j].0 += Vec3::new(v[0], v[1], v[2]);
            acc[j].1 += 1.0;
        }
    }
    let centers = acc.iter().map(|(p, n)| if *n > 0.0 { *p / *n } else { Vec3::ZERO }).collect();
    Some(CharData { verts, bones, clips, centers, color, normal })
}

pub fn chars() -> &'static Vec<CharData> {
    static DATA: OnceLock<Vec<CharData>> = OnceLock::new();
    DATA.get_or_init(|| {
        let src: [(&[u8], &'static [u8], &'static [u8]); 17] = [
            (include_bytes!("../assets/chars/bila.ocm"), include_bytes!("../assets/chars/bila_c.jpg"), include_bytes!("../assets/chars/bila_n.jpg")),
            (include_bytes!("../assets/chars/femme_wax.ocm"), include_bytes!("../assets/chars/femme_wax_c.jpg"), include_bytes!("../assets/chars/femme_wax_n.jpg")),
            (include_bytes!("../assets/chars/homme_boubou.ocm"), include_bytes!("../assets/chars/homme_boubou_c.jpg"), include_bytes!("../assets/chars/homme_boubou_n.jpg")),
            (include_bytes!("../assets/chars/jeune_homme.ocm"), include_bytes!("../assets/chars/jeune_homme_c.jpg"), include_bytes!("../assets/chars/jeune_homme_n.jpg")),
            (include_bytes!("../assets/chars/femme_ville.ocm"), include_bytes!("../assets/chars/femme_ville_c.jpg"), include_bytes!("../assets/chars/femme_ville_n.jpg")),
            (include_bytes!("../assets/chars/h_jeune.ocm"), include_bytes!("../assets/chars/h_jeune_c.jpg"), include_bytes!("../assets/chars/h_jeune_n.jpg")),
            (include_bytes!("../assets/chars/h_ouvrier.ocm"), include_bytes!("../assets/chars/h_ouvrier_c.jpg"), include_bytes!("../assets/chars/h_ouvrier_n.jpg")),
            (include_bytes!("../assets/chars/h_cadre.ocm"), include_bytes!("../assets/chars/h_cadre_c.jpg"), include_bytes!("../assets/chars/h_cadre_n.jpg")),
            (include_bytes!("../assets/chars/h_ancien.ocm"), include_bytes!("../assets/chars/h_ancien_c.jpg"), include_bytes!("../assets/chars/h_ancien_n.jpg")),
            (include_bytes!("../assets/chars/h_sportif.ocm"), include_bytes!("../assets/chars/h_sportif_c.jpg"), include_bytes!("../assets/chars/h_sportif_n.jpg")),
            (include_bytes!("../assets/chars/f_etudiante.ocm"), include_bytes!("../assets/chars/f_etudiante_c.jpg"), include_bytes!("../assets/chars/f_etudiante_n.jpg")),
            (include_bytes!("../assets/chars/f_cadre.ocm"), include_bytes!("../assets/chars/f_cadre_c.jpg"), include_bytes!("../assets/chars/f_cadre_n.jpg")),
            (include_bytes!("../assets/chars/f_sportive.ocm"), include_bytes!("../assets/chars/f_sportive_c.jpg"), include_bytes!("../assets/chars/f_sportive_n.jpg")),
            (include_bytes!("../assets/chars/f_maman.ocm"), include_bytes!("../assets/chars/f_maman_c.jpg"), include_bytes!("../assets/chars/f_maman_n.jpg")),
            (include_bytes!("../assets/chars/f_grandmere.ocm"), include_bytes!("../assets/chars/f_grandmere_c.jpg"), include_bytes!("../assets/chars/f_grandmere_n.jpg")),
            (include_bytes!("../assets/chars/bila_costume.ocm"), include_bytes!("../assets/chars/bila_costume_c.jpg"), include_bytes!("../assets/chars/bila_costume_n.jpg")),
            (include_bytes!("../assets/chars/bila_veste.ocm"), include_bytes!("../assets/chars/bila_veste_c.jpg"), include_bytes!("../assets/chars/bila_veste_n.jpg")),
        ];
        src.iter().filter_map(|(m, c, n)| parse(m, c, n)).collect()
    })
}

/// World position (centre of the hand) and transform of a bone, to hold a weapon.
pub fn hand_world(model: usize, base: Mat4, anim: Anim, t: f32, bone: &str) -> Option<(Vec3, Mat4)> {
    let c = chars().get(model)?;
    let b = c.bone(bone)?;
    let m = base * c.sample(anim, t)[b];
    Some((m.transform_point3(c.centers[b]), m))
}

/// One character to draw.
#[derive(Clone, Copy)]
pub struct CInst {
    pub model: usize,
    pub mat: Mat4,
    pub anim: Anim,
    pub t: f32,
}

struct Gpu {
    vao: glow::VertexArray,
    count: i32,
    base: glow::Texture,
    normal: glow::Texture,
}

pub struct Skinned {
    list: Vec<Gpu>,
    prog: glow::Program,
}

const VS: &str = r#"
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_nrm;
layout(location=2) in vec2 a_uv;
layout(location=3) in vec4 a_joints;
layout(location=4) in vec4 a_weights;
uniform mat4 u_vp; uniform mat4 u_model; uniform mat4 u_lvp;
uniform mat4 u_bones[40];
out vec3 v_pos; out vec3 v_nrm; out vec2 v_uv; out vec4 v_sc;
void main() {
    mat4 s = u_bones[int(a_joints.x)] * a_weights.x + u_bones[int(a_joints.y)] * a_weights.y
           + u_bones[int(a_joints.z)] * a_weights.z + u_bones[int(a_joints.w)] * a_weights.w;
    vec4 wp = u_model * s * vec4(a_pos, 1.0);
    v_pos = wp.xyz;
    v_nrm = normalize(mat3(u_model) * mat3(s) * a_nrm);
    v_uv = a_uv;
    v_sc = u_lvp * vec4(wp.xyz + v_nrm * 0.03, 1.0);
    gl_Position = u_vp * wp;
}
"#;

const FS: &str = r#"
precision highp float;
precision highp sampler2DShadow;
uniform sampler2D u_base; uniform sampler2D u_nmap; uniform sampler2DShadow u_shadow;
uniform vec3 u_cam; uniform vec3 u_sun; uniform vec3 u_sun_col; uniform vec3 u_amb; uniform vec3 u_fogc;
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
    vec3 col = t.rgb * (u_amb * (0.85 + 0.25 * n.y) + u_sun_col * diff * sh);
    vec3 v = normalize(u_cam - v_pos);
    col += u_amb * pow(1.0 - max(dot(n, v), 0.0), 3.0) * 0.25;
    col += u_sun_col * 0.08 * pow(max(dot(n, normalize(v + u_sun)), 0.0), 24.0) * sh;
    float d = length(v_pos - u_cam);
    float f = clamp((d - u_fog * 0.25) / (u_fog * 0.75), 0.0, 1.0);
    frag = vec4(mix(col, u_fogc, f * f * (3.0 - 2.0 * f)), 1.0);
}
"#;

unsafe fn texture(gl: &glow::Context, bytes: &[u8]) -> Result<glow::Texture, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?.to_rgba8();
    let t = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(t));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, img.width() as i32, img.height() as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(img.as_raw())));
    gl.generate_mipmap(glow::TEXTURE_2D);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(t)
}

impl Skinned {
    pub fn new(gl: &glow::Context, compile: impl Fn(&str, &str) -> Result<glow::Program, String>) -> Result<Skinned, String> {
        let prog = compile(VS, FS)?;
        let mut list = Vec::new();
        for c in chars() {
            unsafe {
                let vao = gl.create_vertex_array()?;
                let vbo = gl.create_buffer()?;
                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                let bytes = std::slice::from_raw_parts(c.verts.as_ptr() as *const u8, c.verts.len() * 4);
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
                for (loc, size, off) in [(0u32, 3i32, 0i32), (1, 3, 3), (2, 2, 6), (3, 4, 8), (4, 4, 12)] {
                    gl.enable_vertex_attrib_array(loc);
                    gl.vertex_attrib_pointer_f32(loc, size, glow::FLOAT, false, 64, off * 4);
                }
                gl.bind_vertex_array(None);
                list.push(Gpu { vao, count: (c.verts.len() / 16) as i32, base: texture(gl, c.color)?, normal: texture(gl, c.normal)? });
            }
        }
        Ok(Skinned { list, prog })
    }

    pub fn draw(&self, gl: &glow::Context, f: &crate::models::Frame, insts: &[CInst], depth_only: bool, shadow_tex: glow::Texture) {
        let data = chars();
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
            let (lm, lb) = (l("u_model"), l("u_bones"));
            let mut buf: Vec<f32> = Vec::with_capacity(40 * 16);
            for i in insts {
                let (Some(g), Some(c)) = (self.list.get(i.model), data.get(i.model)) else { continue };
                gl.active_texture(glow::TEXTURE4);
                gl.bind_texture(glow::TEXTURE_2D, Some(g.base));
                gl.active_texture(glow::TEXTURE5);
                gl.bind_texture(glow::TEXTURE_2D, Some(g.normal));
                gl.uniform_matrix_4_f32_slice(lm.as_ref(), false, &i.mat.to_cols_array());
                buf.clear();
                for m in c.sample(i.anim, i.t).iter().take(40) {
                    buf.extend_from_slice(&m.to_cols_array());
                }
                while buf.len() < 40 * 16 {
                    buf.extend_from_slice(&Mat4::IDENTITY.to_cols_array());
                }
                gl.uniform_matrix_4_f32_slice(lb.as_ref(), false, &buf);
                gl.bind_vertex_array(Some(g.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, g.count);
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
