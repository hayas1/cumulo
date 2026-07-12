const LABEL_MIN: f64 = 9.0;
const LABEL_MAX: f64 = 24.0;

const THRESHOLD_STEPS: [f64; 4] = [0.0, 1.8, 4.5, 7.5];

const NODE_LABEL_THRESHOLD_FACTOR: f64 = 1.3;
const LABEL_FADE_IN_SPAN: f64 = 0.6;
const LABEL_FADE_OUT_RATIO: f64 = 0.75;

#[derive(Clone, Copy, Debug)]
pub struct Lod {
    pub max_depth: usize,
    pub layout_scale: f64,
}

impl Lod {
    pub fn new(max_depth: usize, layout_scale: f64) -> Self {
        Lod {
            max_depth,
            layout_scale: layout_scale.max(1e-6),
        }
    }

    pub fn cluster_threshold(&self, depth: usize) -> f64 {
        let raw = THRESHOLD_STEPS
            .get(depth)
            .copied()
            .unwrap_or(THRESHOLD_STEPS[THRESHOLD_STEPS.len() - 1]);
        raw / self.layout_scale
    }

    pub fn node_threshold(&self) -> f64 {
        self.cluster_threshold(self.max_depth)
    }

    pub fn node_label_threshold(&self) -> f64 {
        self.node_threshold() * NODE_LABEL_THRESHOLD_FACTOR
    }

    pub fn cluster_visible(&self, depth: usize, scale: f64) -> bool {
        scale >= self.cluster_threshold(depth)
    }

    pub fn cluster_label_opacity(&self, depth: usize, scale: f64) -> f64 {
        let show = self.cluster_threshold(depth);
        let mut hide = self.cluster_threshold(depth + 1);
        if hide == 0.0 {
            hide = self.node_threshold();
        }
        if scale < show {
            return 0.0;
        }
        let fade_in = (show + LABEL_FADE_IN_SPAN).min((show + hide) / 2.0);
        let fade_out = (hide * LABEL_FADE_OUT_RATIO).max(fade_in);
        if scale < fade_in {
            return (scale - show) / (fade_in - show);
        }
        if scale < fade_out {
            return 1.0;
        }
        if scale < hide {
            return 1.0 - (scale - fade_out) / (hide - fade_out);
        }
        0.0
    }

    pub fn node_visible(&self, scale: f64) -> bool {
        scale >= self.node_threshold()
    }

    pub fn node_label_visible(&self, scale: f64) -> bool {
        scale >= self.node_label_threshold()
    }

    pub fn text_font_size(base_fs: f64, max_fs: f64, scale: f64) -> f64 {
        let screen = (base_fs * scale).clamp(LABEL_MIN, max_fs);
        screen / scale
    }

    pub fn default_max_fs() -> f64 {
        LABEL_MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeper_clusters_have_higher_thresholds() {
        let lod = Lod::new(2, 1.0);
        assert_eq!(lod.cluster_threshold(0), 0.0);
        assert!(lod.cluster_threshold(1) < lod.cluster_threshold(2));
        assert!(lod.cluster_visible(0, 0.0));
        assert!(!lod.cluster_visible(1, 0.0));
    }

    #[test]
    fn larger_layout_scale_lowers_thresholds() {
        let narrow = Lod::new(2, 1.0);
        let wide = Lod::new(2, 2.0);
        assert!(wide.cluster_threshold(1) < narrow.cluster_threshold(1));
    }

    #[test]
    fn cluster_label_fades_in_and_out() {
        let lod = Lod::new(2, 1.0);
        assert_eq!(lod.cluster_label_opacity(1, 1.0), 0.0);
        assert_eq!(lod.cluster_label_opacity(1, 3.0), 1.0);
        assert_eq!(lod.cluster_label_opacity(1, 10.0), 0.0);
    }

    #[test]
    fn text_font_size_clamps_onscreen_size() {
        assert_eq!(Lod::text_font_size(5.0, 24.0, 1.0), 9.0);
        assert!((Lod::text_font_size(10.0, 24.0, 10.0) - 2.4).abs() < 1e-9);
    }

    #[test]
    fn zero_layout_scale_does_not_panic() {
        let lod = Lod::new(1, 0.0);
        let _ = lod.cluster_threshold(1);
        let _ = lod.node_threshold();
    }
}
