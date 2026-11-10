use crate::{
    model::{Face, Question},
    theme::Theme,
};
use egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Vec2, epaint::Vertex};
use glam::{Mat3, Quat, Vec3};
use rand::Rng;
use std::{
    f32::consts::TAU,
    time::{Duration, Instant},
};

const EYE: Vec3 = Vec3::new(4.8, 3.9, 4.8);
const IDLE_RESUME_DELAY: Duration = Duration::from_millis(1500);
const SHELL_RADIUS: f32 = 0.1275;
const SHELL_CORE: f32 = 1.0 - SHELL_RADIUS;

pub struct IdleCube {
    orientation: Quat,
    velocity: Vec3,
    target_velocity: Vec3,
    last_frame: Instant,
    last_drag: Option<Instant>,
    next_direction: Instant,
}
impl Default for IdleCube {
    fn default() -> Self {
        let now = Instant::now();
        let velocity = Vec3::new(0.1, 0.28, 0.06);
        Self {
            orientation: Quat::IDENTITY,
            velocity,
            target_velocity: velocity,
            last_frame: now,
            last_drag: None,
            next_direction: now + Duration::from_secs(4),
        }
    }
}
impl IdleCube {
    pub fn rotation(&self) -> Mat3 {
        Mat3::from_quat(self.orientation)
    }
    #[cfg(feature = "qa")]
    pub fn front_view(&mut self) {
        let forward = EYE.normalize();
        let right = Vec3::Y.cross(forward).normalize();
        let up = forward.cross(right).normalize();
        self.orientation = Quat::from_mat3(&Mat3::from_cols(right, up, forward))
            * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 - 0.06);
        self.last_drag = Some(Instant::now());
    }
    pub fn update(&mut self, delta: Vec2, now: Instant, rng: &mut impl Rng) {
        let dt = now
            .saturating_duration_since(self.last_frame)
            .as_secs_f32()
            .min(0.05);
        self.last_frame = now;
        if delta != Vec2::ZERO {
            let forward = EYE.normalize();
            let right = Vec3::Y.cross(forward).normalize();
            let up = forward.cross(right).normalize();
            self.orientation = (Quat::from_axis_angle(up, delta.x * 0.008)
                * Quat::from_axis_angle(right, delta.y * 0.008)
                * self.orientation)
                .normalize();
            self.last_drag = Some(now);
            return;
        }
        let auto_dt = match self.last_drag {
            Some(last) => dt.min(
                now.saturating_duration_since(last + IDLE_RESUME_DELAY)
                    .as_secs_f32(),
            ),
            None => dt,
        };
        if auto_dt == 0.0 {
            return;
        }
        if now >= self.next_direction {
            self.target_velocity = Vec3::new(
                rng.random_range(-0.3..0.3),
                rng.random_range(-0.3..0.3),
                rng.random_range(-0.3..0.3),
            )
            .normalize_or_zero()
                * rng.random_range(0.25..0.45);
            self.next_direction = now + Duration::from_secs_f32(rng.random_range(3.0..6.0));
        }
        self.velocity = self
            .velocity
            .lerp(self.target_velocity, 1.0 - (-auto_dt / 1.5).exp());
        self.orientation =
            (Quat::from_scaled_axis(self.velocity * auto_dt) * self.orientation).normalize();
    }
}

#[derive(Clone, Copy)]
struct SurfaceVertex {
    p: Vec3,
    n: Vec3,
}
struct Triangle {
    vertices: [SurfaceVertex; 3],
    face: Face,
}

/// Colored molded shells share a rounded boundary, as on a stickerless cubie.
pub struct CubeRenderer {
    shell: Vec<Triangle>,
}
impl Default for CubeRenderer {
    fn default() -> Self {
        let mut shell = Vec::new();
        let samples = [
            -1.0,
            -0.995,
            -0.985,
            -0.970,
            -0.945,
            -0.92,
            -0.89,
            -SHELL_CORE,
            -0.87,
            -0.85,
            -0.80,
            -0.4,
            0.0,
            0.4,
            0.80,
            0.85,
            0.87,
            SHELL_CORE,
            0.89,
            0.92,
            0.945,
            0.970,
            0.985,
            0.995,
            1.0,
        ];
        for face in Face::ALL {
            let (normal, u, v) = basis(face);
            for i in 0..samples.len() - 1 {
                for j in 0..samples.len() - 1 {
                    let vertex = |a: f32, b: f32| {
                        let point = normal + u * a + v * b;
                        let core = point.clamp(Vec3::splat(-SHELL_CORE), Vec3::splat(SHELL_CORE));
                        let n = (point - core).normalize();
                        SurfaceVertex {
                            p: core + n * SHELL_RADIUS,
                            n,
                        }
                    };
                    let a = vertex(samples[i], samples[j]);
                    let b = vertex(samples[i + 1], samples[j]);
                    let c = vertex(samples[i + 1], samples[j + 1]);
                    let d = vertex(samples[i], samples[j + 1]);
                    shell.push(Triangle {
                        vertices: [a, b, c],
                        face,
                    });
                    shell.push(Triangle {
                        vertices: [a, c, d],
                        face,
                    });
                }
            }
        }
        Self { shell }
    }
}
fn basis(face: Face) -> (Vec3, Vec3, Vec3) {
    match face {
        Face::Red => (Vec3::X, Vec3::Y, Vec3::Z),
        Face::Orange => (-Vec3::X, Vec3::Y, -Vec3::Z),
        Face::White => (Vec3::Y, Vec3::Z, Vec3::X),
        Face::Yellow => (-Vec3::Y, Vec3::Z, -Vec3::X),
        Face::Green => (Vec3::Z, Vec3::X, Vec3::Y),
        Face::Blue => (-Vec3::Z, Vec3::X, -Vec3::Y),
    }
}
impl CubeRenderer {
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        question: Option<&Question>,
        palette: &[[u8; 3]; 6],
        idle_rotation: Mat3,
    ) {
        let center = rect.center() + Vec2::new(0.0, -8.0);
        let scale = (rect.height() / 300.0).min(rect.width() / 440.0);
        shadow(
            painter,
            Pos2::new(center.x, rect.bottom() - 14.0),
            Vec2::new(151.0, 28.0) * scale,
        );
        let rotation = if question.is_some() {
            Mat3::IDENTITY
        } else {
            idle_rotation
        };
        let eye = EYE;
        let forward = eye.normalize();
        let right = Vec3::Y.cross(forward).normalize();
        let up = forward.cross(right).normalize();
        let focal = (rect.height() * 2.8).min(rect.width() * 1.9);
        let project = |p: Vec3| {
            let relative = p - eye;
            let depth = -relative.dot(forward);
            (
                Pos2::new(
                    center.x + focal * relative.dot(right) / depth,
                    center.y - focal * relative.dot(up) / depth,
                ),
                depth,
            )
        };
        let mut triangles: Vec<_> = self
            .shell
            .iter()
            .map(|t| Triangle {
                vertices: t.vertices.map(|v| SurfaceVertex {
                    p: rotation * v.p,
                    n: rotation * v.n,
                }),
                face: t.face,
            })
            .filter(|t| t.vertices.iter().map(|v| v.n.dot(eye - v.p)).sum::<f32>() > 0.0)
            .collect();
        triangles.sort_by(|a, b| {
            let da: f32 = a.vertices.iter().map(|v| project(v.p).1).sum();
            let db: f32 = b.vertices.iter().map(|v| project(v.p).1).sum();
            db.total_cmp(&da)
        });
        let light = Vec3::new(-3.0, 7.0, 5.0).normalize();
        let fill = Vec3::new(6.0, 1.0, -1.0).normalize();
        let mut mesh = Mesh::default();
        for triangle in triangles {
            let face = triangle.face;
            let color = match question {
                Some(q) if face.axis() == q.hidden => [166, 171, 177],
                Some(q) if face.sign() == 1 => palette[q.faces[face.axis()].index()],
                Some(q) if face == Face::Yellow => palette[q.bottom().index()],
                Some(q) => palette[q.faces[face.axis()].opposite().index()],
                None => palette[face.index()],
            };
            let base = mesh.vertices.len() as u32;
            for vertex in triangle.vertices {
                let diffuse = vertex.n.dot(light).max(0.0);
                let filling = vertex.n.dot(fill).max(0.0);
                let view = (eye - vertex.p).normalize();
                let halfway = (light + view).normalize();
                let gloss = vertex.n.dot(halfway).max(0.0).powf(34.0) * 0.18;
                let luminance = 0.78 + diffuse * 0.19 + filling * 0.075 + vertex.p.y * 0.018;
                let rgb =
                    color.map(|c| (c as f32 * luminance + gloss * 255.0).clamp(0.0, 255.0) as u8);
                mesh.vertices.push(Vertex {
                    pos: project(vertex.p).0,
                    uv: egui::epaint::WHITE_UV,
                    color: Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                });
            }
            mesh.indices.extend([base, base + 1, base + 2]);
        }
        painter.add(Shape::mesh(mesh));
    }
}

fn shadow(p: &Painter, center: Pos2, radius: Vec2) {
    let mut mesh = Mesh::default();
    let theme = Theme::get(p.ctx());
    for (ring, (fraction, alpha)) in [
        (0.0_f32, 48_u8),
        (0.2, 40),
        (0.4, 26),
        (0.6, 12),
        (0.8, 3),
        (1.0, 0),
    ]
    .iter()
    .enumerate()
    {
        for i in 0..64 {
            let t = i as f32 / 64.0 * TAU;
            mesh.vertices.push(Vertex {
                pos: center + Vec2::new(t.cos() * radius.x, t.sin() * radius.y) * *fraction,
                uv: egui::epaint::WHITE_UV,
                color: Color32::from_black_alpha(if theme.dark {
                    (*alpha as u16 * 2).min(255) as u8
                } else {
                    *alpha
                }),
            });
            if ring > 0 {
                let a = ((ring - 1) * 64 + i) as u32;
                let b = ((ring - 1) * 64 + (i + 1) % 64) as u32;
                let c = (ring * 64 + (i + 1) % 64) as u32;
                let d = (ring * 64 + i) as u32;
                mesh.indices.extend([a, b, c, a, c, d]);
            }
        }
    }
    p.add(Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn dragging_holds_orientation_for_1500ms_then_resumes_without_a_jump() {
        let mut rng = StdRng::seed_from_u64(13);
        let mut cube = IdleCube::default();
        let now = cube.last_frame;
        cube.update(Vec2::new(80.0, -45.0), now, &mut rng);
        let dragged = cube.orientation;
        assert!(dragged.angle_between(Quat::IDENTITY).abs() > 0.1);
        cube.update(Vec2::ZERO, now + Duration::from_millis(1499), &mut rng);
        assert_eq!(cube.orientation, dragged);
        cube.update(Vec2::ZERO, now + Duration::from_millis(1500), &mut rng);
        assert_eq!(cube.orientation, dragged);
        cube.update(Vec2::ZERO, now + Duration::from_millis(1550), &mut rng);
        let resumed = cube.orientation;
        assert_ne!(resumed, dragged);
        assert!(resumed.angle_between(dragged).abs() < 0.03);
        cube.update(
            Vec2::new(-20.0, 60.0),
            now + Duration::from_secs(3),
            &mut rng,
        );
        let second_drag = cube.orientation;
        cube.update(Vec2::ZERO, now + Duration::from_millis(4499), &mut rng);
        assert_eq!(cube.orientation, second_drag);
        assert!((cube.orientation.length() - 1.0).abs() < 1e-5);
    }
}
