'use strict';

// ── グローバルコールバックオブジェクト（Rustから差し込まれる） ─────────────
window.__cumuloCallbacks = {};

// ── 状態 ──────────────────────────────────────────────────────────────────────
let resources = [];
let filteredIds = new Set();
let zoomAxes = ['vendor', 'service', 'resource_type'];
let currentScale = 1;
let initialized = false;

// ── D3セレクション ────────────────────────────────────────────────────────────
let svg, g, zoom;
let W = 900, H = 600;

// ── ベンダー / クラスタカラー ─────────────────────────────────────────────────
const COLORS = {
  GCP:     '#1D9E75',
  AWS:     '#378ADD',
  Azure:   '#7F77DD',
  Datadog: '#E24B4A',
  Sentry:  '#9B6DD6',
};
const RESOURCE_TYPE_COLORS = {
  BigQuery:   '#1D9E75',
  Aurora:     '#378ADD',
  RDS:        '#378ADD',
  GCS:        '#0F6E56',
  APM:        '#E24B4A',
  CosmosDB:   '#7F77DD',
  EC2:        '#BA7517',
  ErrorTrack: '#9B6DD6',
};
const DEFAULT_COLOR = '#6b8099';

function clusterColor(key) {
  return COLORS[key] || RESOURCE_TYPE_COLORS[key] || DEFAULT_COLOR;
}

// ── パブリックAPI ─────────────────────────────────────────────────────────────

window.cumuloInitMap = function (svgId) {
  const el = document.getElementById(svgId);
  if (!el) return;

  const rect = el.getBoundingClientRect();
  W = rect.width || 900;
  H = rect.height || 600;

  svg = d3.select('#' + svgId)
    .attr('width', W)
    .attr('height', H);

  // 背景クリックでズームリセット
  svg.on('click', function (event) {
    if (event.target === svg.node()) {
      zoomToFit();
    }
  });

  zoom = d3.zoom()
    .scaleExtent([0.25, 14])
    .on('zoom', onZoom);

  svg.call(zoom);

  g = svg.append('g').attr('class', 'zoom-group');

  initialized = true;
  render();
};

window.cumuloUpdateResources = function (json) {
  try {
    resources = JSON.parse(json);
    filteredIds = new Set(resources.map(r => r.id));
    if (initialized) render();
  } catch (e) { console.error('cumuloUpdateResources', e); }
};

window.cumuloUpdateFilter = function (selectedJson) {
  try {
    // selectedJson は [["key","val"], ...] 形式（Rustのtupleシリアライズ）
    const selected = JSON.parse(selectedJson);
    if (!selected || selected.length === 0) {
      filteredIds = new Set(resources.map(r => r.id));
    } else {
      filteredIds = new Set(
        resources
          .filter(r => selected.every(([k, v]) => r.attrs && r.attrs[k] === v))
          .map(r => r.id)
      );
    }
    if (initialized) updateFilterOpacity();
  } catch (e) { console.error('cumuloUpdateFilter', e); }
};

window.cumuloUpdateZoomAxes = function (axesJson) {
  try {
    zoomAxes = JSON.parse(axesJson);
    if (initialized) {
      render();
      zoomToFit();
    }
  } catch (e) { console.error('cumuloUpdateZoomAxes', e); }
};

window.cumuloZoomToFit = function () { zoomToFit(); };

window.cumuloZoomIn = function () {
  if (svg) svg.transition().duration(300).call(zoom.scaleBy, 1.5);
};

window.cumuloZoomOut = function () {
  if (svg) svg.transition().duration(300).call(zoom.scaleBy, 1 / 1.5);
};

// ── 内部: ズームハンドラ ──────────────────────────────────────────────────────

function onZoom(event) {
  const { transform } = event;
  g.attr('transform', transform);
  currentScale = transform.k;
  updateLOD(transform.k);
}

function updateLOD(scale) {
  if (!g) return;
  // ミニノード: scale > 1.5 で表示
  g.selectAll('.mini-node')
    .style('opacity', scale > 1.5 ? 1 : 0)
    .style('pointer-events', scale > 1.5 ? 'auto' : 'none');

  // ノードラベル: scale > 3.5 で表示
  g.selectAll('.node-label')
    .style('opacity', scale > 3.5 ? 1 : 0);

  // クラスタラベル: ズームするほどフェードアウト
  g.selectAll('.cluster-label')
    .style('opacity', Math.max(0, 1 - (scale - 1.2) / 1.0));

  // クラスタカウント: ラベルと同じく
  g.selectAll('.cluster-count')
    .style('opacity', Math.max(0, 1 - (scale - 1.2) / 1.0));
}

// ── 内部: ズーム遷移 ─────────────────────────────────────────────────────────

function zoomToCluster(d) {
  // クラスタがビューポートの70%を占めるスケールを計算
  const scale = (Math.min(W, H) * 0.7) / (d.r * 2);
  const tx = W / 2 - scale * d.x;
  const ty = H / 2 - scale * d.y;

  svg.transition().duration(800).ease(d3.easeCubicInOut)
    .call(zoom.transform, d3.zoomIdentity.translate(tx, ty).scale(scale));

  const cb = window.__cumuloCallbacks.onZoomLevelChange;
  if (cb) cb(1);
}

function zoomToFit() {
  if (!svg) return;
  svg.transition().duration(600).ease(d3.easeCubicInOut)
    .call(zoom.transform, d3.zoomIdentity);

  const cb = window.__cumuloCallbacks.onZoomLevelChange;
  if (cb) cb(0);
}

// ── 内部: レンダリング ────────────────────────────────────────────────────────

function render() {
  if (!initialized || !g) return;

  g.selectAll('*').remove();

  if (resources.length === 0) return;

  const axis = zoomAxes[0] || 'vendor';
  const groups = d3.group(resources, r => (r.attrs && r.attrs[axis]) || 'その他');

  // クラスタデータ生成
  const clusterData = Array.from(groups, ([key, items]) => {
    const totalFreq = items.reduce((s, r) => s + (r.freq || 1), 0);
    return { key, items, totalFreq, x: 0, y: 0, r: 0 };
  });

  // 半径計算（面積がfreq合計に比例）
  const maxFreq = Math.max(...clusterData.map(c => c.totalFreq), 1);
  const maxR = Math.min(W, H) * 0.2;
  const minR = 55;
  clusterData.forEach(c => {
    c.r = minR + (maxR - minR) * Math.sqrt(c.totalFreq / maxFreq);
  });

  // フォースシミュレーション前の初期配置（円形）
  clusterData.forEach((d, i) => {
    const angle = (i / clusterData.length) * 2 * Math.PI - Math.PI / 2;
    const spread = Math.min(W, H) * 0.3;
    d.x = W / 2 + Math.cos(angle) * spread;
    d.y = H / 2 + Math.sin(angle) * spread;
  });

  // フォースシミュレーション（静的に収束させる）
  const sim = d3.forceSimulation(clusterData)
    .force('center', d3.forceCenter(W / 2, H / 2).strength(0.25))
    .force('charge', d3.forceManyBody().strength(-80))
    .force('collide', d3.forceCollide(d => d.r + 28).strength(0.92))
    .stop();

  for (let i = 0; i < 250; i++) sim.tick();

  // ── クラスタグループ描画 ────────────────────────────────────────────────
  const clusterG = g.selectAll('.cluster')
    .data(clusterData, d => d.key)
    .join('g')
    .attr('class', 'cluster')
    .attr('transform', d => `translate(${d.x},${d.y})`);

  // クラスタ背景円
  clusterG.append('circle')
    .attr('class', 'cluster-bg')
    .attr('r', d => d.r)
    .attr('fill', d => clusterColor(d.key) + '28')
    .attr('stroke', d => clusterColor(d.key))
    .attr('stroke-width', 2)
    .style('cursor', 'pointer')
    .on('click', (event, d) => {
      event.stopPropagation();
      zoomToCluster(d);
    });

  // クラスタラベル（主名）
  clusterG.append('text')
    .attr('class', 'cluster-label')
    .attr('text-anchor', 'middle')
    .attr('dy', '0.2em')
    .attr('fill', d => clusterColor(d.key))
    .attr('font-size', d => Math.max(15, d.r / 4))
    .attr('font-weight', 700)
    .attr('font-family', 'system-ui, sans-serif')
    .attr('pointer-events', 'none')
    .text(d => d.key);

  // クラスタカウント
  clusterG.append('text')
    .attr('class', 'cluster-count')
    .attr('text-anchor', 'middle')
    .attr('dy', d => d.r / 4 + 20)
    .attr('fill', d => clusterColor(d.key))
    .attr('fill-opacity', 0.65)
    .attr('font-size', 11)
    .attr('font-family', 'system-ui, sans-serif')
    .attr('pointer-events', 'none')
    .text(d => `${d.items.length} リソース`);

  // ── ミニノード（クラスタ内の個別リソース） ────────────────────────────
  clusterG.each(function (cluster) {
    const el = d3.select(this);
    const n = cluster.items.length;

    cluster.items.forEach((r, i) => {
      // ゴールデンアングル螺旋でリソースをクラスタ内に配置
      const goldenAngle = 137.508 * (Math.PI / 180);
      const angle = i * goldenAngle;
      const spread = cluster.r * 0.68;
      const rawDist = 0.28 * spread * Math.sqrt(i + 1);
      const dist = Math.min(rawDist, spread);
      const mx = Math.cos(angle) * dist;
      const my = Math.sin(angle) * dist;

      // freqに応じてノードサイズを調整
      const nodeR = Math.max(4, Math.min(15, (r.freq || 1) * 0.9 + 3));

      const nodeG = el.append('g')
        .attr('class', 'mini-node')
        .attr('transform', `translate(${mx},${my})`)
        .style('opacity', 0)          // LODで制御
        .style('cursor', 'pointer')
        .datum(r);

      nodeG.append('circle')
        .attr('class', 'mini-node-circle')
        .attr('r', nodeR)
        .attr('fill', clusterColor(cluster.key))
        .attr('fill-opacity', filteredIds.has(r.id) ? 0.85 : 0.12)
        .attr('stroke', '#0d1117')
        .attr('stroke-width', 1);

      nodeG.append('text')
        .attr('class', 'node-label')
        .attr('text-anchor', 'middle')
        .attr('dy', nodeR + 11)
        .attr('font-size', 9)
        .attr('font-family', 'monospace')
        .attr('fill', '#c9d1d9')
        .attr('pointer-events', 'none')
        .style('opacity', 0)
        .text(r.name);

      nodeG.on('click', (event, resource) => {
        event.stopPropagation();
        const cb = window.__cumuloCallbacks.onResourceSelect;
        if (cb) cb(resource.id);
      });
    });
  });

  // 現在のscaleに合わせてLOD適用
  updateLOD(currentScale);
  updateFilterOpacity();
}

// ── 内部: フィルター透明度更新 ───────────────────────────────────────────────

function updateFilterOpacity() {
  if (!g) return;

  // ミニノード
  g.selectAll('.mini-node').each(function (d) {
    if (!d) return;
    d3.select(this).select('.mini-node-circle')
      .transition().duration(250)
      .attr('fill-opacity', filteredIds.has(d.id) ? 0.85 : 0.1);
  });

  // クラスタ背景
  g.selectAll('.cluster-bg').each(function (d) {
    if (!d) return;
    const hasMatch = d.items.some(r => filteredIds.has(r.id));
    d3.select(this)
      .transition().duration(250)
      .attr('fill', clusterColor(d.key) + (hasMatch ? '30' : '08'))
      .attr('stroke-opacity', hasMatch ? 1 : 0.2);
  });
}
