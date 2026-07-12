#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub r: f64,
}

impl Body {
    pub fn at(x: f64, y: f64, r: f64) -> Self {
        Body {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            r,
        }
    }
}

const REPULSION: f64 = -30.0;
const COLLISION_PADDING: f64 = 5.0;
const COLLISION_STRENGTH: f64 = 0.9;
const CENTER_STRENGTH: f64 = 0.3;
const VELOCITY_DECAY: f64 = 0.4;
const DISTANCE_MIN_SQ: f64 = 1.0;
const TICKS: usize = 250;
const BOUND_RADIUS_RATIO: f64 = 0.82;
const ALPHA_MIN: f64 = 0.001;
const ALPHA_DECAY_TICKS: f64 = 300.0;
const JITTER_SCALE: f64 = 1e-6;

struct Lcg {
    state: u64,
}

impl Lcg {
    const A: u64 = 1_664_525;
    const C: u64 = 1_013_904_223;
    const M: u64 = 4_294_967_296;

    fn new() -> Self {
        Lcg { state: 1 }
    }

    fn next_unit(&mut self) -> f64 {
        self.state = (Self::A.wrapping_mul(self.state).wrapping_add(Self::C)) % Self::M;
        self.state as f64 / Self::M as f64
    }

    fn jitter(&mut self) -> f64 {
        (self.next_unit() - 0.5) * JITTER_SCALE
    }
}

pub struct Simulation {
    bodies: Vec<Body>,
    cx: f64,
    cy: f64,
    bound_r: Option<f64>,
    alpha: f64,
    alpha_decay: f64,
    rng: Lcg,
}

impl Simulation {
    pub fn new(bodies: Vec<Body>, cx: f64, cy: f64, bound_r: Option<f64>) -> Self {
        let alpha_decay = 1.0 - ALPHA_MIN.powf(1.0 / ALPHA_DECAY_TICKS);
        Simulation {
            bodies,
            cx,
            cy,
            bound_r,
            alpha: 1.0,
            alpha_decay,
            rng: Lcg::new(),
        }
    }

    pub fn run(mut self) -> Vec<Body> {
        for _ in 0..TICKS {
            self.tick();
        }
        self.bodies
    }

    fn tick(&mut self) {
        self.alpha += (0.0 - self.alpha) * self.alpha_decay;

        self.apply_center();
        self.apply_repulsion();
        self.apply_collision();
        self.apply_bound();
        self.integrate();
    }

    fn apply_center(&mut self) {
        let n = self.bodies.len();
        if n == 0 {
            return;
        }
        let (mut sx, mut sy) = (0.0, 0.0);
        for b in &self.bodies {
            sx += b.x;
            sy += b.y;
        }
        let shift_x = (sx / n as f64 - self.cx) * CENTER_STRENGTH;
        let shift_y = (sy / n as f64 - self.cy) * CENTER_STRENGTH;
        for b in &mut self.bodies {
            b.x -= shift_x;
            b.y -= shift_y;
        }
    }

    fn apply_repulsion(&mut self) {
        let n = self.bodies.len();
        let alpha = self.alpha;
        for i in 0..n {
            let (xi, yi) = (self.bodies[i].x, self.bodies[i].y);
            let (mut acc_vx, mut acc_vy) = (0.0, 0.0);
            for j in 0..n {
                if i == j {
                    continue;
                }
                let mut dx = self.bodies[j].x - xi;
                let mut dy = self.bodies[j].y - yi;
                if dx == 0.0 {
                    dx = self.rng.jitter();
                }
                if dy == 0.0 {
                    dy = self.rng.jitter();
                }
                let mut l = dx * dx + dy * dy;
                if l < DISTANCE_MIN_SQ {
                    l = (DISTANCE_MIN_SQ * l).sqrt();
                }
                let w = REPULSION * alpha / l;
                acc_vx += dx * w;
                acc_vy += dy * w;
            }
            self.bodies[i].vx += acc_vx;
            self.bodies[i].vy += acc_vy;
        }
    }

    fn apply_collision(&mut self) {
        let n = self.bodies.len();
        for i in 0..n {
            let ri = self.bodies[i].r + COLLISION_PADDING;
            let ri2 = ri * ri;
            let xi = self.bodies[i].x + self.bodies[i].vx;
            let yi = self.bodies[i].y + self.bodies[i].vy;
            for j in (i + 1)..n {
                let rj = self.bodies[j].r + COLLISION_PADDING;
                let r = ri + rj;
                let mut x = xi - (self.bodies[j].x + self.bodies[j].vx);
                let mut y = yi - (self.bodies[j].y + self.bodies[j].vy);
                let mut l = x * x + y * y;
                if l >= r * r {
                    continue;
                }
                if x == 0.0 {
                    x = self.rng.jitter();
                    l += x * x;
                }
                if y == 0.0 {
                    y = self.rng.jitter();
                    l += y * y;
                }
                let dist = l.sqrt();
                let factor = (r - dist) / dist * COLLISION_STRENGTH;
                x *= factor;
                y *= factor;
                let rj2 = rj * rj;
                let share_i = rj2 / (ri2 + rj2);
                self.bodies[i].vx += x * share_i;
                self.bodies[i].vy += y * share_i;
                let share_j = 1.0 - share_i;
                self.bodies[j].vx -= x * share_j;
                self.bodies[j].vy -= y * share_j;
            }
        }
    }

    fn apply_bound(&mut self) {
        let Some(bound_r) = self.bound_r else {
            return;
        };
        for b in &mut self.bodies {
            let dx = b.x - self.cx;
            let dy = b.y - self.cy;
            let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
            let limit = bound_r * BOUND_RADIUS_RATIO - b.r;
            if limit > 0.0 && dist > limit {
                b.x = self.cx + dx / dist * limit;
                b.y = self.cy + dy / dist * limit;
            }
        }
    }

    fn integrate(&mut self) {
        for b in &mut self.bodies {
            b.vx *= VELOCITY_DECAY;
            b.x += b.vx;
            b.vy *= VELOCITY_DECAY;
            b.y += b.vy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: &Body, b: &Body) -> f64 {
        ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
    }

    #[test]
    fn collision_separates_overlapping_bodies() {
        let bodies = vec![Body::at(100.0, 100.0, 20.0), Body::at(110.0, 100.0, 20.0)];
        let out = Simulation::new(bodies, 105.0, 100.0, None).run();
        assert!(
            dist(&out[0], &out[1]) >= 40.0,
            "expected separation, got {}",
            dist(&out[0], &out[1])
        );
    }

    #[test]
    fn simulation_is_deterministic() {
        let make = || vec![Body::at(100.0, 100.0, 10.0), Body::at(100.0, 100.0, 10.0)];
        let a = Simulation::new(make(), 100.0, 100.0, None).run();
        let b = Simulation::new(make(), 100.0, 100.0, None).run();
        assert_eq!(a, b);
    }

    #[test]
    fn bound_keeps_bodies_inside() {
        let cx = 200.0;
        let cy = 200.0;
        let bound_r = 100.0;
        let bodies = vec![
            Body::at(400.0, 200.0, 5.0),
            Body::at(200.0, 400.0, 5.0),
            Body::at(0.0, 200.0, 5.0),
        ];
        let out = Simulation::new(bodies, cx, cy, Some(bound_r)).run();
        for b in &out {
            let d = ((b.x - cx).powi(2) + (b.y - cy).powi(2)).sqrt();
            assert!(d <= bound_r * 0.82 + 1e-6, "body escaped bound: {d}");
        }
    }

    #[test]
    fn empty_simulation_is_noop() {
        let out = Simulation::new(vec![], 0.0, 0.0, None).run();
        assert!(out.is_empty());
    }
}
