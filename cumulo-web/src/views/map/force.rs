//! 力学（force-directed）シミュレーション。クラスタ配置に使う。
//!
//! 中心引力 / 斥力（repulsion）/ 衝突（collision）の 3 力と、半径境界の押し戻しを、
//! alpha 減衰・速度減衰つきの反復積分で安定化する。
//! jitter（微小ゆらぎ）は固定 seed の線形合同法で決定論的にし、結果をテスト可能にする。

/// シミュレーション対象の 1 粒子。`x,y` は配置座標、`vx,vy` は速度、`r` は半径。
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

// ── シミュレーションが固定で使う係数 ──────────────────────────────────────────
/// 斥力の強さ（負＝反発）。
const REPULSION: f64 = -30.0;
/// 衝突半径に加える余白 px。
const COLLISION_PADDING: f64 = 5.0;
/// 衝突解消の強さ。
const COLLISION_STRENGTH: f64 = 0.9;
/// 中心への引き寄せの強さ。
const CENTER_STRENGTH: f64 = 0.3;
/// 速度減衰係数（毎 tick 乗算）。
const VELOCITY_DECAY: f64 = 0.4;
/// 斥力の最小距離二乗（近接時の発散を防ぐ下限）。
const DISTANCE_MIN_SQ: f64 = 1.0;
/// シミュレーションの反復回数。
const TICKS: usize = 250;
/// 半径境界: 中心からこの比 × bound_r を超えたノードを円内へ戻す。
const BOUND_RADIUS_RATIO: f64 = 0.82;
/// alpha の最小値と、そこまで減衰させる目安 tick 数（alpha_decay の算出に使う）。
const ALPHA_MIN: f64 = 0.001;
const ALPHA_DECAY_TICKS: f64 = 300.0;
/// jitter（微小ゆらぎ）の振幅。
const JITTER_SCALE: f64 = 1e-6;

/// 微小ゆらぎ（jitter）に使う線形合同法乱数。seed=1 から決定論的に進める。
struct Lcg {
    state: u64,
}

impl Lcg {
    const A: u64 = 1_664_525;
    const C: u64 = 1_013_904_223;
    const M: u64 = 4_294_967_296; // 2^32

    fn new() -> Self {
        Lcg { state: 1 }
    }

    fn next_unit(&mut self) -> f64 {
        self.state = (Self::A.wrapping_mul(self.state).wrapping_add(Self::C)) % Self::M;
        self.state as f64 / Self::M as f64
    }

    /// 座標が完全一致したときに微小なずれを与える（重なり時の発散回避）。
    fn jitter(&mut self) -> f64 {
        (self.next_unit() - 0.5) * JITTER_SCALE
    }
}

/// 力学シミュレーション本体。初期位置を設定した Body 列を受け取り、
/// 250 tick 回したあとの座標を返す。
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
    /// `bodies` は初期位置 (x,y) を設定済みであること。`bound_r` が Some のときだけ
    /// 半径 bound_r*0.82 の円内へ各ノードを押し戻す（半径境界）。
    pub fn new(bodies: Vec<Body>, cx: f64, cy: f64, bound_r: Option<f64>) -> Self {
        // alpha_decay = 1 - alpha_min^(1/ticks): tick ごとに alpha を幾何的に減衰させる。
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

    /// 250 tick 走らせ、確定した配置を返す。
    pub fn run(mut self) -> Vec<Body> {
        for _ in 0..TICKS {
            self.tick();
        }
        self.bodies
    }

    fn tick(&mut self) {
        // alphaTarget=0 へ向けて減衰
        self.alpha += (0.0 - self.alpha) * self.alpha_decay;

        // 力は中心引力 → 斥力 → 衝突 → 半径境界の順で適用し、最後に積分する。
        self.apply_center();
        self.apply_repulsion();
        self.apply_collision();
        self.apply_bound();
        self.integrate();
    }

    /// 中心引力: 重心を (cx,cy) へ寄せるよう全ノードを平行移動する（速度には触れない）。
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

    /// 斥力: 全ペアの反発（O(n^2) 直接計算）。強さは負なので反発になる。
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

    /// 衝突: 予測位置 (x+vx) を使って重なりを解消する。半径は r + padding。
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

    /// 半径境界: 中心から半径 bound_r*0.82-r を超えたノードを円周上へ戻す。
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

    /// 速度減衰と位置更新。
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

    // 重なった同半径の 2 ノードは衝突力で離れ、最終的に半径和+padding 以上に開く。
    #[test]
    fn collision_separates_overlapping_bodies() {
        let bodies = vec![Body::at(100.0, 100.0, 20.0), Body::at(110.0, 100.0, 20.0)];
        let out = Simulation::new(bodies, 105.0, 100.0, None).run();
        // 半径 20+20、padding 5*2 を含めても少なくとも素の半径和 40 以上には離れる
        assert!(
            dist(&out[0], &out[1]) >= 40.0,
            "expected separation, got {}",
            dist(&out[0], &out[1])
        );
    }

    // 同じ入力からは同じ配置になる（jitter 乱数を含め決定論的）。
    #[test]
    fn simulation_is_deterministic() {
        let make = || vec![Body::at(100.0, 100.0, 10.0), Body::at(100.0, 100.0, 10.0)];
        let a = Simulation::new(make(), 100.0, 100.0, None).run();
        let b = Simulation::new(make(), 100.0, 100.0, None).run();
        assert_eq!(a, b);
    }

    // bound_r 指定時、全ノードは中心から bound_r*0.82 を超えない位置に収まる。
    #[test]
    fn bound_keeps_bodies_inside() {
        let cx = 200.0;
        let cy = 200.0;
        let bound_r = 100.0;
        // 外側に散らした初期配置
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

    // 空入力でもパニックしない。
    #[test]
    fn empty_simulation_is_noop() {
        let out = Simulation::new(vec![], 0.0, 0.0, None).run();
        assert!(out.is_empty());
    }
}
