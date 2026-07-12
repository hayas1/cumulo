use std::time::Duration;

use leptos::prelude::*;

use super::layout::Bounds;

pub const SCALE_MIN: f64 = 0.2;
pub const SCALE_MAX: f64 = 20.0;

const ZOOM_STEP_FACTOR: f64 = 1.5;
const FIT_PADDING: f64 = 40.0;
const FOCUS_FILL_RATIO: f64 = 0.85;

const ZOOM_STEP_DURATION: Duration = Duration::from_millis(300);
const FIT_DURATION: Duration = Duration::from_millis(600);
const FOCUS_DURATION: Duration = Duration::from_millis(800);

const WHEEL_UNIT_LINE: f64 = 0.05;
const WHEEL_UNIT_PIXEL: f64 = 0.002;
const WHEEL_PINCH_GAIN: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
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

    pub fn to_svg(self) -> String {
        format!("translate({},{}) scale({})", self.x, self.y, self.scale)
    }

    fn clamp_scale(scale: f64) -> f64 {
        scale.clamp(SCALE_MIN, SCALE_MAX)
    }

    pub fn scale_to_about(&self, target: f64, px: f64, py: f64) -> Transform {
        let target = Self::clamp_scale(target);
        let mx = (px - self.x) / self.scale;
        let my = (py - self.y) / self.scale;
        Transform {
            scale: target,
            x: px - target * mx,
            y: py - target * my,
        }
    }

    pub fn scale_by_about(&self, factor: f64, px: f64, py: f64) -> Transform {
        self.scale_to_about(self.scale * factor, px, py)
    }

    pub fn translated(&self, dx: f64, dy: f64) -> Transform {
        Transform {
            scale: self.scale,
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn interpolate_view(from: Transform, to: Transform, t: f64, w: f64, h: f64) -> Transform {
        if t <= 0.0 {
            return from;
        }
        if t >= 1.0 {
            return to;
        }
        let cx0 = (w / 2.0 - from.x) / from.scale;
        let cy0 = (h / 2.0 - from.y) / from.scale;
        let cx1 = (w / 2.0 - to.x) / to.scale;
        let cy1 = (h / 2.0 - to.y) / to.scale;
        let scale = from.scale * (to.scale / from.scale).powf(t);
        let cx = cx0 + (cx1 - cx0) * t;
        let cy = cy0 + (cy1 - cy0) * t;
        Transform {
            scale,
            x: w / 2.0 - scale * cx,
            y: h / 2.0 - scale * cy,
        }
    }

    fn ease_cubic_in_out(t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            t * t * t / 2.0
        } else {
            let t = t - 2.0;
            (t * t * t + 2.0) / 2.0
        }
    }

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

    pub fn wheel_factor(delta_y: f64, delta_mode: u32, ctrl_key: bool) -> f64 {
        let unit = match delta_mode {
            1 => WHEEL_UNIT_LINE,
            0 => WHEEL_UNIT_PIXEL,
            _ => 1.0,
        };
        let ctrl = if ctrl_key { WHEEL_PINCH_GAIN } else { 1.0 };
        2_f64.powf(-delta_y * unit * ctrl)
    }

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pan {
    start_x: f64,
    start_y: f64,
    start: Transform,
}

impl Pan {
    pub const DRAG_THRESHOLD: f64 = 3.0;

    pub fn begin(x: f64, y: f64, start: Transform) -> Self {
        Pan {
            start_x: x,
            start_y: y,
            start,
        }
    }

    pub fn is_drag(&self, x: f64, y: f64) -> bool {
        (x - self.start_x).hypot(y - self.start_y) > Self::DRAG_THRESHOLD
    }

    pub fn transform_at(&self, x: f64, y: f64) -> Transform {
        self.start.translated(x - self.start_x, y - self.start_y)
    }
}

#[derive(Clone, Copy)]
pub struct ZoomController {
    pub transform: RwSignal<Transform>,
    pub viewport: RwSignal<(f64, f64)>,
    pub content_bounds: RwSignal<Option<Bounds>>,
    anim_gen: RwSignal<u64>,
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
        if self.anim_gen.get_untracked() != gen {
            return;
        }
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

    pub fn set_immediate(&self, transform: Transform) {
        self.anim_gen.set(self.anim_gen.get_untracked() + 1);
        self.transform.set(transform);
    }

    pub fn zoom_in(&self) {
        let (w, h) = self.viewport_size();
        let target =
            self.transform
                .get_untracked()
                .scale_by_about(ZOOM_STEP_FACTOR, w / 2.0, h / 2.0);
        self.animate_to(target, ZOOM_STEP_DURATION);
    }

    pub fn zoom_out(&self) {
        let (w, h) = self.viewport_size();
        let target =
            self.transform
                .get_untracked()
                .scale_by_about(1.0 / ZOOM_STEP_FACTOR, w / 2.0, h / 2.0);
        self.animate_to(target, ZOOM_STEP_DURATION);
    }

    pub fn zoom_to_fit(&self) {
        let (w, h) = self.viewport_size();
        let target = match self.content_bounds.get_untracked() {
            Some(bounds) => Transform::fit(bounds, w, h, FIT_PADDING),
            None => Transform::IDENTITY,
        };
        self.animate_to(target, FIT_DURATION);
    }

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

    #[test]
    fn scale_about_keeps_anchor_point_fixed() {
        let t = Transform::IDENTITY;
        let (px, py) = (300.0, 200.0);
        let z = t.scale_to_about(4.0, px, py);
        let screen_x = z.scale * ((px - t.x) / t.scale) + z.x;
        let screen_y = z.scale * ((py - t.y) / t.scale) + z.y;
        assert!((screen_x - px).abs() < 1e-9);
        assert!((screen_y - py).abs() < 1e-9);
    }

    #[test]
    fn scale_is_clamped_to_extent() {
        let t = Transform::IDENTITY;
        assert_eq!(t.scale_to_about(1000.0, 0.0, 0.0).scale, SCALE_MAX);
        assert_eq!(t.scale_to_about(0.001, 0.0, 0.0).scale, SCALE_MIN);
    }

    #[test]
    fn fit_centers_content() {
        let bounds = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 100.0,
            max_y: 100.0,
        };
        let t = Transform::fit(bounds, 400.0, 400.0, 0.0);
        assert!((t.scale * 50.0 + t.x - 200.0).abs() < 1e-9);
        assert!((t.scale * 50.0 + t.y - 200.0).abs() < 1e-9);
    }

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

    #[test]
    fn ease_endpoints_and_midpoint() {
        assert!((Transform::ease_cubic_in_out(0.0)).abs() < 1e-9);
        assert!((Transform::ease_cubic_in_out(1.0) - 1.0).abs() < 1e-9);
        assert!((Transform::ease_cubic_in_out(0.5) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn wheel_scroll_direction_zooms_in_and_out() {
        assert!(Transform::wheel_factor(-100.0, 0, false) > 1.0);
        assert!(Transform::wheel_factor(100.0, 0, false) < 1.0);
    }

    #[test]
    fn wheel_pinch_is_ten_times_stronger() {
        let plain = Transform::wheel_factor(-5.0, 0, false);
        let pinch = Transform::wheel_factor(-5.0, 0, true);
        assert!((pinch - plain.powi(10)).abs() < 1e-9);
        assert!(pinch > plain);
    }

    #[test]
    fn wheel_delta_mode_changes_unit() {
        let pixel = Transform::wheel_factor(-1.0, 0, false);
        let line = Transform::wheel_factor(-1.0, 1, false);
        assert!(line > pixel);
    }

    #[test]
    fn focus_node_centers_and_scales_clicked_cluster() {
        let (w, h) = (400.0, 300.0);
        let (ax, ay, r) = (100.0, 50.0, 10.0);
        let t = Transform::focus_node(ax, ay, r, w, h);
        assert!((t.scale * ax + t.x - w / 2.0).abs() < 1e-9);
        assert!((t.scale * ay + t.y - h / 2.0).abs() < 1e-9);
        assert!((t.scale - (h * 0.85 / (2.0 * r))).abs() < 1e-9);
    }

    #[test]
    fn focus_node_scale_is_clamped() {
        assert_eq!(
            Transform::focus_node(0.0, 0.0, 0.01, 400.0, 300.0).scale,
            SCALE_MAX
        );
        assert_eq!(
            Transform::focus_node(0.0, 0.0, 100000.0, 400.0, 300.0).scale,
            SCALE_MIN
        );
    }

    #[test]
    fn pan_distinguishes_click_from_drag() {
        let pan = Pan::begin(100.0, 100.0, Transform::IDENTITY);
        assert!(!pan.is_drag(101.0, 101.0));
        assert!(pan.is_drag(120.0, 100.0));
    }

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
        assert!((mid.scale - 2.0).abs() < 1e-9);
    }

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
        let center_model = (w / 2.0 - mid.x) / mid.scale;
        assert!((center_model - 162.5).abs() < 1e-9);
    }

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
