//! ズーム拡大率に応じた詳細度（Level of Detail）の計算。
//!
//! クラスタ/ノードの表示しきい値・ラベルのフェード・テキストの逆スケール補正を純粋関数で求める。
//! Leptos 側はこの計算結果を要素の opacity / font-size へ反映する。

/// ラベルのオンスクリーン下限・既定上限 px。
const LABEL_MIN: f64 = 9.0;
const LABEL_MAX: f64 = 24.0;

/// 階層深さに配分するしきい値の基準列（layout_scale で正規化する前の生値）。
const THRESHOLD_STEPS: [f64; 4] = [0.0, 1.8, 4.5, 7.5];

/// リソース名ラベルが見え始める拡大率＝node_threshold の何倍か。
const NODE_LABEL_THRESHOLD_FACTOR: f64 = 1.3;
/// クラスタラベルのフェードイン幅の上限（show からの加算）。
const LABEL_FADE_IN_SPAN: f64 = 0.6;
/// クラスタラベルのフェードアウト開始位置（hide に対する割合）。
const LABEL_FADE_OUT_RATIO: f64 = 0.75;

/// レイアウトの広がり（layout_scale）と最大深さ（max_depth）に基づく LOD 計算器。
#[derive(Clone, Copy, Debug)]
pub struct Lod {
    pub max_depth: usize,
    pub layout_scale: f64,
}

impl Lod {
    pub fn new(max_depth: usize, layout_scale: f64) -> Self {
        // layout_scale=0 だと 0 除算になるため下限を設ける（空レイアウト時の保険）。
        Lod {
            max_depth,
            layout_scale: layout_scale.max(1e-6),
        }
    }

    /// depth のクラスタが見え始めるしきい値。layout_scale で割って正規化する。
    pub fn cluster_threshold(&self, depth: usize) -> f64 {
        let raw = THRESHOLD_STEPS
            .get(depth)
            .copied()
            .unwrap_or(THRESHOLD_STEPS[THRESHOLD_STEPS.len() - 1]);
        raw / self.layout_scale
    }

    /// リソース円（mini-node）が見え始める拡大率しきい値＝最深クラスタの 1 段先。
    pub fn node_threshold(&self) -> f64 {
        self.cluster_threshold(self.max_depth)
    }

    /// リソース名ラベルが見え始める拡大率しきい値。
    pub fn node_label_threshold(&self) -> f64 {
        self.node_threshold() * NODE_LABEL_THRESHOLD_FACTOR
    }

    /// depth のクラスタ本体を表示するか。
    pub fn cluster_visible(&self, depth: usize, scale: f64) -> bool {
        scale >= self.cluster_threshold(depth)
    }

    /// depth のクラスタラベル/件数の不透明度（フェードイン・アウト込み）。
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

    /// リソース円を表示するか。
    pub fn node_visible(&self, scale: f64) -> bool {
        scale >= self.node_threshold()
    }

    /// リソース名ラベルを表示するか。
    pub fn node_label_visible(&self, scale: f64) -> bool {
        scale >= self.node_label_threshold()
    }

    /// テキストのオンスクリーンサイズを読める帯に収め、拡大率で逆補正したフォントサイズを返す。
    /// `base*scale` が本来の画面サイズ。LABEL_MIN〜max_fs にクランプしてから /scale する。
    pub fn text_font_size(base_fs: f64, max_fs: f64, scale: f64) -> f64 {
        let screen = (base_fs * scale).clamp(LABEL_MIN, max_fs);
        screen / scale
    }

    /// ノードラベル等で max_fs 未指定のときの既定上限。
    pub fn default_max_fs() -> f64 {
        LABEL_MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // depth 0 は常時表示（しきい値 0）、深いほどしきい値が上がる。
    #[test]
    fn deeper_clusters_have_higher_thresholds() {
        let lod = Lod::new(2, 1.0);
        assert_eq!(lod.cluster_threshold(0), 0.0);
        assert!(lod.cluster_threshold(1) < lod.cluster_threshold(2));
        assert!(lod.cluster_visible(0, 0.0));
        assert!(!lod.cluster_visible(1, 0.0));
    }

    // layout_scale が大きいほど同じ詳細が低い倍率で見える（しきい値が下がる）。
    #[test]
    fn larger_layout_scale_lowers_thresholds() {
        let narrow = Lod::new(2, 1.0);
        let wide = Lod::new(2, 2.0);
        assert!(wide.cluster_threshold(1) < narrow.cluster_threshold(1));
    }

    // クラスタラベルは show 未満で 0、フェード帯の中央付近で 1。
    #[test]
    fn cluster_label_fades_in_and_out() {
        let lod = Lod::new(2, 1.0);
        // depth 1: show=1.8, hide=4.5
        assert_eq!(lod.cluster_label_opacity(1, 1.0), 0.0); // show 未満
        assert_eq!(lod.cluster_label_opacity(1, 3.0), 1.0); // 帯の中
        assert_eq!(lod.cluster_label_opacity(1, 10.0), 0.0); // hide 超
    }

    // テキストは画面サイズ下限を割らず、上限も超えない（/scale 逆補正後の値で検証）。
    #[test]
    fn text_font_size_clamps_onscreen_size() {
        // base=5, scale=1 → screen=5 だが下限 9 にクランプ → font=9
        assert_eq!(Lod::text_font_size(5.0, 24.0, 1.0), 9.0);
        // base=10, scale=10 → screen=100 だが上限 24 → font=24/10=2.4
        assert!((Lod::text_font_size(10.0, 24.0, 10.0) - 2.4).abs() < 1e-9);
    }

    // 空レイアウト相当（layout_scale=0）でも 0 除算でパニックしない。
    #[test]
    fn zero_layout_scale_does_not_panic() {
        let lod = Lod::new(1, 0.0);
        let _ = lod.cluster_threshold(1);
        let _ = lod.node_threshold();
    }
}
