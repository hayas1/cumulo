//! パン・ズームのビュー変換の状態と操作。
//!
//! 画面変換は `translate(x,y) scale(s)` の [`Transform`] で表し、拡大率を範囲内へクランプする。
//! プログラム的なズーム遷移は requestAnimationFrame で 3 次イージング補間する。

use std::time::Duration;

use leptos::prelude::*;

use super::layout::Bounds;

/// 拡大率の下限・上限（許容スケール範囲）。
pub const SCALE_MIN: f64 = 0.2;
pub const SCALE_MAX: f64 = 20.0;

/// +/- ボタン 1 回あたりのズーム倍率。
const ZOOM_STEP_FACTOR: f64 = 1.5;
/// 全体表示でビューポート端に残す余白 px。
const FIT_PADDING: f64 = 40.0;
/// focus_node でクラスタが占める画面の割合（短辺比）。
const FOCUS_FILL_RATIO: f64 = 0.85;

/// プログラム的ズーム遷移の所要時間。
const ZOOM_STEP_DURATION: Duration = Duration::from_millis(300);
const FIT_DURATION: Duration = Duration::from_millis(600);
const FOCUS_DURATION: Duration = Duration::from_millis(800);

/// ホイール量 → 倍率指数の単位係数（deltaMode 別）。
const WHEEL_UNIT_LINE: f64 = 0.05;
const WHEEL_UNIT_PIXEL: f64 = 0.002;
/// ピンチ（ctrlKey 付き wheel）の増感倍率。
const WHEEL_PINCH_GAIN: f64 = 10.0;

/// SVG の `translate(x,y) scale(s)` に対応する画面変換。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// 拡大率（スケール係数）。world→screen の一様スケール。
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Transform::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        scale: 1.0,
        x: 0.0,
        y: 0.0,
    };

    /// `<g>` の transform 属性値。
    pub fn to_svg(self) -> String {
        format!("translate({},{}) scale({})", self.x, self.y, self.scale)
    }

    fn clamp_scale(scale: f64) -> f64 {
        scale.clamp(SCALE_MIN, SCALE_MAX)
    }

    /// 画面点 (px,py) を固定したまま拡大率を target（クランプ後）へ変更する。
    pub fn scale_to_about(&self, target: f64, px: f64, py: f64) -> Transform {
        let target = Self::clamp_scale(target);
        // モデル座標（変換前）を固定点として保つ
        let mx = (px - self.x) / self.scale;
        let my = (py - self.y) / self.scale;
        Transform {
            scale: target,
            x: px - target * mx,
            y: py - target * my,
        }
    }

    /// 画面点 (px,py) を中心に拡大率を factor 倍する。
    pub fn scale_by_about(&self, factor: f64, px: f64, py: f64) -> Transform {
        self.scale_to_about(self.scale * factor, px, py)
    }

    /// 画面上の平行移動量 (dx,dy) を加える。
    pub fn translated(&self, dx: f64, dy: f64) -> Transform {
        Transform {
            scale: self.scale,
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    /// 2 つの変換の間を「視覚的に一様」に補間する。
    /// 拡大率は対数（幾何）補間し、ビューポート中心が指すモデル座標を線形に動かす。
    /// 拡大率を線形補間すると拡大の見かけ速度が不均一になりカクついて見えるため。
    /// `t` は呼び出し側でイージング済みの 0〜1。端点では from / to を厳密に返す。
    pub fn interpolate_view(from: Transform, to: Transform, t: f64, w: f64, h: f64) -> Transform {
        if t <= 0.0 {
            return from;
        }
        if t >= 1.0 {
            return to;
        }
        // 各変換でビューポート中心が指すモデル座標
        let cx0 = (w / 2.0 - from.x) / from.scale;
        let cy0 = (h / 2.0 - from.y) / from.scale;
        let cx1 = (w / 2.0 - to.x) / to.scale;
        let cy1 = (h / 2.0 - to.y) / to.scale;
        // 拡大率は幾何補間、中心は線形補間
        let scale = from.scale * (to.scale / from.scale).powf(t);
        let cx = cx0 + (cx1 - cx0) * t;
        let cy = cy0 + (cy1 - cy0) * t;
        Transform {
            scale,
            x: w / 2.0 - scale * cx,
            y: h / 2.0 - scale * cy,
        }
    }

    /// 3 次の ease-in-out（加減速を滑らかにするイージング）。
    fn ease_cubic_in_out(t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            t * t * t / 2.0
        } else {
            let t = t - 2.0;
            (t * t * t + 2.0) / 2.0
        }
    }

    /// 内容 bounds を viewport(w,h) に余白 pad で収める変換。
    pub fn fit(bounds: Bounds, w: f64, h: f64, pad: f64) -> Transform {
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Transform::IDENTITY;
        }
        let scale = Self::clamp_scale(
            ((w - pad * 2.0) / bounds.width()).min((h - pad * 2.0) / bounds.height()),
        );
        let (cx, cy) = bounds.center();
        Transform {
            scale,
            x: w / 2.0 - scale * cx,
            y: h / 2.0 - scale * cy,
        }
    }

    /// ホイール/ピンチ 1 イベント分の拡大率（倍率）。
    /// deltaMode で単位を切り替え、ピンチ（ctrlKey 付き wheel）は ×10 で増感する。
    /// delta_y が負（上スクロール）で 1 より大きい倍率（ズームイン）になる。
    pub fn wheel_factor(delta_y: f64, delta_mode: u32, ctrl_key: bool) -> f64 {
        let unit = match delta_mode {
            1 => WHEEL_UNIT_LINE,
            0 => WHEEL_UNIT_PIXEL,
            _ => 1.0, // page
        };
        let ctrl = if ctrl_key { WHEEL_PINCH_GAIN } else { 1.0 };
        2_f64.powf(-delta_y * unit * ctrl)
    }

    /// 絶対座標 (ax,ay) を中心に半径 r のクラスタを画面の約 85% に収める変換。
    pub fn focus_node(ax: f64, ay: f64, r: f64, w: f64, h: f64) -> Transform {
        let natural = (w.min(h) * FOCUS_FILL_RATIO) / (r * 2.0);
        let scale = Self::clamp_scale(natural);
        Transform {
            scale,
            x: w / 2.0 - scale * ax,
            y: h / 2.0 - scale * ay,
        }
    }
}

/// ドラッグ（パン）の進行状態。クリックとドラッグを区別し、開始時の変換を基準に平行移動する。
/// 純粋な値オブジェクトなので状態機械をイベント処理から切り離してテストできる。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pan {
    start_x: f64,
    start_y: f64,
    start: Transform,
}

impl Pan {
    /// この距離（px）を超えて初めてドラッグとみなす。クリックの誤判定を防ぐ。
    pub const DRAG_THRESHOLD: f64 = 3.0;

    /// pointerdown 位置と、その時点の変換から開始する。
    pub fn begin(x: f64, y: f64, start: Transform) -> Self {
        Pan {
            start_x: x,
            start_y: y,
            start,
        }
    }

    /// 開始点から DRAG_THRESHOLD を超えて動いたか（＝ドラッグ確定）。
    pub fn is_drag(&self, x: f64, y: f64) -> bool {
        (x - self.start_x).hypot(y - self.start_y) > Self::DRAG_THRESHOLD
    }

    /// 現在のポインタ位置に対応する平行移動後の変換。
    pub fn transform_at(&self, x: f64, y: f64) -> Transform {
        self.start.translated(x - self.start_x, y - self.start_y)
    }
}

/// ズーム状態と操作をまとめ、Controls / MapCanvas で共有する。
/// すべての状態はシグナルなので `Copy` で持ち回せる。
#[derive(Clone, Copy)]
pub struct ZoomController {
    pub transform: RwSignal<Transform>,
    /// SVG のビューポート (w,h)。getBoundingClientRect で更新する。
    pub viewport: RwSignal<(f64, f64)>,
    /// 現在のレイアウト内容の境界。レイアウト再計算時に更新する。
    pub content_bounds: RwSignal<Option<Bounds>>,
    /// 進行中アニメーションの世代。新しい遷移が始まると古い rAF ループを止める。
    anim_gen: RwSignal<u64>,
    /// ホイール/ピンチを rAF で間引くための、未反映の累積倍率・アンカー・予約フラグ。
    wheel_accum: RwSignal<f64>,
    wheel_anchor: RwSignal<(f64, f64)>,
    wheel_pending: RwSignal<bool>,
}

impl ZoomController {
    pub fn new() -> Self {
        ZoomController {
            transform: RwSignal::new(Transform::IDENTITY),
            viewport: RwSignal::new((900.0, 600.0)),
            content_bounds: RwSignal::new(None),
            anim_gen: RwSignal::new(0),
            wheel_accum: RwSignal::new(1.0),
            wheel_anchor: RwSignal::new((0.0, 0.0)),
            wheel_pending: RwSignal::new(false),
        }
    }

    /// ホイール/ピンチ 1 イベント分の倍率を画面点 (px,py) 中心で適用する。
    /// 同一フレーム内の連続イベントは倍率を掛け合わせ、rAF で 1 回だけ反映する。
    /// トラックパッドはフレーム内に複数回発火するため、これで拡大率変化に伴う
    /// ノード再描画をフレームあたり高々 1 回に抑える。
    pub fn zoom_by(&self, factor: f64, px: f64, py: f64) {
        self.wheel_accum.update(|f| *f *= factor);
        self.wheel_anchor.set((px, py));
        if self.wheel_pending.get_untracked() {
            return;
        }
        self.wheel_pending.set(true);
        let this = *self;
        request_animation_frame(move || {
            let f = this.wheel_accum.get_untracked();
            let (px, py) = this.wheel_anchor.get_untracked();
            this.wheel_accum.set(1.0);
            this.wheel_pending.set(false);
            let next = this.transform.get_untracked().scale_by_about(f, px, py);
            this.set_immediate(next);
        });
    }

    fn viewport_size(&self) -> (f64, f64) {
        self.viewport.get_untracked()
    }

    /// 現在の変換から target へ duration かけて補間遷移する。
    fn animate_to(&self, target: Transform, duration: Duration) {
        let gen = self.anim_gen.get_untracked() + 1;
        self.anim_gen.set(gen);
        let from = self.transform.get_untracked();
        let (w, h) = self.viewport_size();
        let start = js_sys::Date::now();
        self.tween_step(from, target, w, h, duration, gen, start);
    }

    #[allow(clippy::too_many_arguments)]
    fn tween_step(
        self,
        from: Transform,
        to: Transform,
        w: f64,
        h: f64,
        duration: Duration,
        gen: u64,
        start: f64,
    ) {
        // 新しい遷移が始まっていたらこのループは破棄する
        if self.anim_gen.get_untracked() != gen {
            return;
        }
        // start / now はブラウザの時計（ms）。経過 / 所要時間で進捗 t を求める。
        let now = js_sys::Date::now();
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let t = if duration_ms <= 0.0 {
            1.0
        } else {
            ((now - start) / duration_ms).clamp(0.0, 1.0)
        };
        let eased = Transform::ease_cubic_in_out(t);
        self.transform
            .set(Transform::interpolate_view(from, to, eased, w, h));
        if t < 1.0 {
            request_animation_frame(move || self.tween_step(from, to, w, h, duration, gen, start));
        }
    }

    /// 進行中アニメーションを止めて即座に変換を設定する（wheel/pan 用）。
    pub fn set_immediate(&self, transform: Transform) {
        self.anim_gen.set(self.anim_gen.get_untracked() + 1);
        self.transform.set(transform);
    }

    /// 中心を保ったまま 1 段ズームイン。
    pub fn zoom_in(&self) {
        let (w, h) = self.viewport_size();
        let target =
            self.transform
                .get_untracked()
                .scale_by_about(ZOOM_STEP_FACTOR, w / 2.0, h / 2.0);
        self.animate_to(target, ZOOM_STEP_DURATION);
    }

    /// 中心を保ったまま 1 段ズームアウト。
    pub fn zoom_out(&self) {
        let (w, h) = self.viewport_size();
        let target =
            self.transform
                .get_untracked()
                .scale_by_about(1.0 / ZOOM_STEP_FACTOR, w / 2.0, h / 2.0);
        self.animate_to(target, ZOOM_STEP_DURATION);
    }

    /// 内容全体が収まるよう遷移する。境界未確定時は identity へ。
    pub fn zoom_to_fit(&self) {
        let (w, h) = self.viewport_size();
        let target = match self.content_bounds.get_untracked() {
            Some(bounds) => Transform::fit(bounds, w, h, FIT_PADDING),
            None => Transform::IDENTITY,
        };
        self.animate_to(target, FIT_DURATION);
    }

    /// クラスタへフォーカスして 1 段掘り下げる。
    pub fn zoom_to_node(&self, ax: f64, ay: f64, r: f64) {
        let (w, h) = self.viewport_size();
        self.animate_to(Transform::focus_node(ax, ay, r, w, h), FOCUS_DURATION);
    }
}

impl Default for ZoomController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // scale_to_about は固定点の画面位置を保つ。
    #[test]
    fn scale_about_keeps_anchor_point_fixed() {
        let t = Transform::IDENTITY;
        let (px, py) = (300.0, 200.0);
        let z = t.scale_to_about(4.0, px, py);
        // 固定点のモデル座標を両変換で screen に戻すと同じ画面位置になる
        let screen_x = z.scale * ((px - t.x) / t.scale) + z.x;
        let screen_y = z.scale * ((py - t.y) / t.scale) + z.y;
        assert!((screen_x - px).abs() < 1e-9);
        assert!((screen_y - py).abs() < 1e-9);
    }

    // 倍率は scaleExtent でクランプされる。
    #[test]
    fn scale_is_clamped_to_extent() {
        let t = Transform::IDENTITY;
        assert_eq!(t.scale_to_about(1000.0, 0.0, 0.0).scale, SCALE_MAX);
        assert_eq!(t.scale_to_about(0.001, 0.0, 0.0).scale, SCALE_MIN);
    }

    // fit は内容中心を viewport 中心に合わせる。
    #[test]
    fn fit_centers_content() {
        let bounds = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 100.0,
            max_y: 100.0,
        };
        let t = Transform::fit(bounds, 400.0, 400.0, 0.0);
        // 中心 (50,50) が viewport 中心 (200,200) に来る
        assert!((t.scale * 50.0 + t.x - 200.0).abs() < 1e-9);
        assert!((t.scale * 50.0 + t.y - 200.0).abs() < 1e-9);
    }

    // 退化した境界は identity を返す（0 除算回避）。
    #[test]
    fn degenerate_bounds_returns_identity() {
        let bounds = Bounds {
            min_x: 10.0,
            min_y: 10.0,
            max_x: 10.0,
            max_y: 10.0,
        };
        assert_eq!(
            Transform::fit(bounds, 400.0, 400.0, 40.0),
            Transform::IDENTITY
        );
    }

    // easeCubicInOut は端点と中点で期待値を取る。
    #[test]
    fn ease_endpoints_and_midpoint() {
        assert!((Transform::ease_cubic_in_out(0.0)).abs() < 1e-9);
        assert!((Transform::ease_cubic_in_out(1.0) - 1.0).abs() < 1e-9);
        assert!((Transform::ease_cubic_in_out(0.5) - 0.5).abs() < 1e-9);
    }

    // 上スクロール（delta_y<0）でズームイン、下スクロールでズームアウト。
    #[test]
    fn wheel_scroll_direction_zooms_in_and_out() {
        assert!(Transform::wheel_factor(-100.0, 0, false) > 1.0);
        assert!(Transform::wheel_factor(100.0, 0, false) < 1.0);
    }

    // ピンチ（ctrlKey 付き wheel）は通常ホイールの 10 倍の指数で増感される。
    // これが抜けるとピンチ拡大が 10 倍遅くなる回帰になる。
    #[test]
    fn wheel_pinch_is_ten_times_stronger() {
        let plain = Transform::wheel_factor(-5.0, 0, false);
        let pinch = Transform::wheel_factor(-5.0, 0, true);
        // factor = 2^(exp)。ctrl は exp を ×10 するので pinch == plain^10。
        assert!((pinch - plain.powi(10)).abs() < 1e-9);
        assert!(pinch > plain);
    }

    // deltaMode（line と pixel）で単位係数が変わる。
    #[test]
    fn wheel_delta_mode_changes_unit() {
        let pixel = Transform::wheel_factor(-1.0, 0, false); // 0.002
        let line = Transform::wheel_factor(-1.0, 1, false); // 0.05
        assert!(line > pixel);
    }

    // クラスタクリックのズーム先: クリック点を画面中心へ写し、半径を約 85% に収める。
    #[test]
    fn focus_node_centers_and_scales_clicked_cluster() {
        let (w, h) = (400.0, 300.0);
        let (ax, ay, r) = (100.0, 50.0, 10.0);
        let t = Transform::focus_node(ax, ay, r, w, h);
        // クリック点が viewport 中心に来る
        assert!((t.scale * ax + t.x - w / 2.0).abs() < 1e-9);
        assert!((t.scale * ay + t.y - h / 2.0).abs() < 1e-9);
        // 倍率は min(w,h)*0.85/(2r)
        assert!((t.scale - (h * 0.85 / (2.0 * r))).abs() < 1e-9);
    }

    // フォーカス倍率も scaleExtent でクランプされる（極小・極大クラスタ）。
    #[test]
    fn focus_node_scale_is_clamped() {
        // 半径が小さすぎると上限へ
        assert_eq!(
            Transform::focus_node(0.0, 0.0, 0.01, 400.0, 300.0).scale,
            SCALE_MAX
        );
        // 半径が大きすぎると下限へ
        assert_eq!(
            Transform::focus_node(0.0, 0.0, 100000.0, 400.0, 300.0).scale,
            SCALE_MIN
        );
    }

    // Pan: しきい値内はクリック扱い、超えるとドラッグ確定。
    #[test]
    fn pan_distinguishes_click_from_drag() {
        let pan = Pan::begin(100.0, 100.0, Transform::IDENTITY);
        // 2px の移動はクリック（ドラッグ未満）
        assert!(!pan.is_drag(101.0, 101.0));
        // 大きく動けばドラッグ
        assert!(pan.is_drag(120.0, 100.0));
    }

    // interpolate_view は端点で from / to を厳密に返す。
    #[test]
    fn interpolate_view_hits_endpoints() {
        let from = Transform {
            scale: 1.0,
            x: 0.0,
            y: 0.0,
        };
        let to = Transform {
            scale: 4.0,
            x: -300.0,
            y: -300.0,
        };
        assert_eq!(
            Transform::interpolate_view(from, to, 0.0, 400.0, 400.0),
            from
        );
        assert_eq!(Transform::interpolate_view(from, to, 1.0, 400.0, 400.0), to);
    }

    // 倍率は幾何補間: 中点で sqrt(k0*k1)（線形補間の (k0+k1)/2 とは異なる）。
    // これがカクつき修正の核心（線形だと 2.5 になってしまう）。
    #[test]
    fn interpolate_view_scales_geometrically() {
        let from = Transform {
            scale: 1.0,
            x: 0.0,
            y: 0.0,
        };
        let to = Transform {
            scale: 4.0,
            x: -300.0,
            y: -300.0,
        };
        let mid = Transform::interpolate_view(from, to, 0.5, 400.0, 400.0);
        assert!((mid.scale - 2.0).abs() < 1e-9); // sqrt(1*4)=2、線形なら 2.5
    }

    // 補間中もビューポート中心が指すモデル座標は from→to の中心を線形に辿る。
    #[test]
    fn interpolate_view_moves_center_smoothly() {
        let (w, h) = (400.0, 400.0);
        let from = Transform {
            scale: 1.0,
            x: 0.0,
            y: 0.0,
        };
        let to = Transform {
            scale: 4.0,
            x: -300.0,
            y: -300.0,
        };
        let mid = Transform::interpolate_view(from, to, 0.5, w, h);
        // from の中心モデル=200、to の中心モデル=(200+300)/4=125、その中点=162.5
        let center_model = (w / 2.0 - mid.x) / mid.scale;
        assert!((center_model - 162.5).abs() < 1e-9);
    }

    // Pan: ドラッグ中はポインタ移動量だけ平行移動する（倍率は不変）。
    #[test]
    fn pan_translates_by_pointer_delta() {
        let start = Transform {
            scale: 2.0,
            x: 10.0,
            y: 20.0,
        };
        let pan = Pan::begin(100.0, 100.0, start);
        let moved = pan.transform_at(130.0, 90.0);
        assert_eq!(moved.scale, 2.0);
        assert_eq!(moved.x, 10.0 + 30.0);
        assert_eq!(moved.y, 20.0 - 10.0);
    }
}
