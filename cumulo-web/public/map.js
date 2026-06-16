'use strict';

// ── グローバルコールバックオブジェクト（Rustから差し込まれる） ─────────────
window.__cumuloCallbacks = {};

// ── 状態 ──────────────────────────────────────────────────────────────────────
let resources = [];
let filteredIds = new Set();
let dimensions = [];          // 完全なディメンション定義（親リンクを辿るのに使う）
let zoomDim = '';             // ズーム軸のディメンションID（例: 'platform'）
let maxDepth = 1;             // フォレストの最大ネスト深さ（LODに使用）
let currentScale = 1;
let initialized = false;

// ── D3セレクション ────────────────────────────────────────────────────────────
let svg, g, zoom;
let W = 900, H = 600;

// ── カラー ────────────────────────────────────────────────────────────────────
let valueColors = {};
const DEFAULT_COLOR = '#6b8099';

function clusterColor(key) {
  return valueColors[key] || DEFAULT_COLOR;
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

  svg.on('click', function (event) {
    if (event.target === svg.node()) zoomToFit(true);
  });

  zoom = d3.zoom()
    .scaleExtent([0.2, 20])
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
    const selected = JSON.parse(selectedJson);
    if (!selected || selected.length === 0) {
      filteredIds = new Set(resources.map(r => r.id));
    } else {
      filteredIds = new Set(
        resources
          .filter(r => selected.every(([k, v]) => {
            const rv = valueOnAxis(r, k);
            if (rv == null) return false;
            // 階層dimは祖先一致も許容（GCP を選ぶと BigQuery もマッチ）
            return ancestryIn(k, rv).includes(v);
          }))
          .map(r => r.id)
      );
    }
    if (initialized) updateFilterOpacity();
  } catch (e) { console.error('cumuloUpdateFilter', e); }
};

window.cumuloUpdateZoomDim = function (json) {
  try {
    const o = JSON.parse(json);
    zoomDim = o.dim || '';
    // ズーム軸の切替では絞り込みは触らない（resetFilter=false）
    if (initialized) { render(); zoomToFit(false); }
  } catch (e) { console.error('cumuloUpdateZoomDim', e); }
};

window.cumuloUpdateDimensions = function (json) {
  try {
    dimensions = JSON.parse(json);
    valueColors = {};
    dimensions.forEach(n => {
      if (n.color) valueColors[n.id] = n.color;
    });
    if (initialized) render();
  } catch (e) { console.error('cumuloUpdateDimensions', e); }
};

// nodeId から根軸ノード（dimId）の手前までの祖先チェーン（自身含む、近い順）を返す。
// 根ノード自身 (dimId) は含まない。
function ancestryIn(dimId, nodeId) {
  const byId = {};
  dimensions.forEach(n => { byId[n.id] = n; });
  const chain = [];
  let cur = nodeId;
  while (cur != null && cur !== dimId && !chain.includes(cur)) {
    chain.push(cur);
    const n = byId[cur];
    cur = n ? (n.parent ?? null) : null;
  }
  return chain;
}

// nodeId が属する軸（parent を辿った最上位の根 id）を返す。
function rootOf(nodeId) {
  const byId = {};
  dimensions.forEach(n => { byId[n.id] = n; });
  let cur = nodeId;
  const seen = new Set();
  while (cur != null && !seen.has(cur)) {
    seen.add(cur);
    const n = byId[cur];
    if (!n || n.parent == null) return cur; // cur が根
    cur = n.parent;
  }
  return null;
}

// リソースの categories（値リスト）から、指定軸の値を返す。なければ undefined。
// 軸（根）は dict のキーではなく root_of で導出する。
function valueOnAxis(r, axisId) {
  return (r.categories || []).find(v => rootOf(v) === axisId);
}

// ノードIDからラベルを返す。見つからなければIDそのまま。
function nodeLabel(id) {
  const n = dimensions.find(d => d.id === id);
  return (n && n.label) ? n.label : id;
}

// リソースの表示名を返す。label がなければ dimensions 値のラベルで代替。
function resourceLabel(r) {
  if (r.label) return r.label;
  const parts = (r.categories || [])
    .map(v => nodeLabel(v))
    .sort();
  return parts.length > 0 ? parts.join(' / ') : '(名前なし)';
}

// ズーム軸ディメンションでの、フォレスト根から葉までの完全パス（上→下）を返す。
// 例: platform で BigQuery → [Cloud, GCP, BigQuery]。フラットdimなら [value]。
function zoomPath(r) {
  if (!zoomDim) return null;
  const leaf = valueOnAxis(r, zoomDim);
  if (leaf == null) return ['その他'];
  return ancestryIn(zoomDim, leaf).reverse();
}

window.cumuloZoomToFit = function () { zoomToFit(true); };

window.cumuloZoomIn = function () {
  if (svg) svg.transition().duration(300).call(zoom.scaleBy, 1.5);
};

window.cumuloZoomOut = function () {
  if (svg) svg.transition().duration(300).call(zoom.scaleBy, 1 / 1.5);
};

// ── ズームハンドラ ────────────────────────────────────────────────────────────

function onZoom(event) {
  const { transform } = event;
  g.attr('transform', transform);
  currentScale = transform.k;
  updateLOD(transform.k);
  applyTextScale(transform.k);
}

// ラベルのオンスクリーンサイズを読める帯（LABEL_MIN〜LABEL_MAX px）に収める。
// base*k が本来の画面サイズ。それをクランプしてから k で逆補正することで、
// 浅い階層では大きくなりすぎず、深い階層でも小さくなりすぎないようにする。
const LABEL_MIN = 9;
const LABEL_MAX = 24;
function applyTextScale(k) {
  if (!g) return;
  g.selectAll('text').each(function () {
    const t = d3.select(this);
    const base = +t.attr('data-fs') || 10;
    const screen = Math.max(LABEL_MIN, Math.min(LABEL_MAX, base * k));
    t.attr('font-size', screen / k);
  });
}

// ── ズーム遷移 ────────────────────────────────────────────────────────────────

// absX, absY はルート座標系での絶対位置
function zoomToNode(absX, absY, r) {
  // クリックしたクラスタが画面の約85%を占める自然なスケール。
  // 深追いせず「1段ぶん」掘り下げる（直下の子が見える程度に収める）。
  const naturalScale = (Math.min(W, H) * 0.85) / (r * 2);
  const scale = Math.max(0.2, Math.min(20, naturalScale));
  const tx = W / 2 - scale * absX;
  const ty = H / 2 - scale * absY;
  svg.transition().duration(800).ease(d3.easeCubicInOut)
    .call(zoom.transform, d3.zoomIdentity.translate(tx, ty).scale(scale));
  const cb = window.__cumuloCallbacks.onZoomLevelChange;
  if (cb) cb(1);
}

// resetFilter=true のとき、ズームアウト（全体表示）に合わせて
// 現在のズーム軸の絞り込みを解除する。ズーム軸切替では false で呼ぶ。
function zoomToFit(resetFilter) {
  if (!svg || !g) return;
  // layout が可変サイズになったため、getBBox でコンテンツ実寸に合わせる。
  // identity リセットだと拡張 layout が画面に収まらない。
  const bbox = g.node().getBBox();
  if (bbox.width > 0 && bbox.height > 0) {
    const pad = 40;
    const k = Math.min((W - pad * 2) / bbox.width, (H - pad * 2) / bbox.height);
    const tx = W / 2 - k * (bbox.x + bbox.width / 2);
    const ty = H / 2 - k * (bbox.y + bbox.height / 2);
    svg.transition().duration(600).ease(d3.easeCubicInOut)
      .call(zoom.transform, d3.zoomIdentity.translate(tx, ty).scale(k));
  } else {
    svg.transition().duration(600).ease(d3.easeCubicInOut)
      .call(zoom.transform, d3.zoomIdentity);
  }
  const cb = window.__cumuloCallbacks.onZoomLevelChange;
  if (cb) cb(0);
  if (resetFilter) {
    const rc = window.__cumuloCallbacks.onZoomReset;
    if (rc) rc();
  }
}

// ── 階層ツリー構築 ────────────────────────────────────────────────────────────
//
// ノード型:
//   { type:'cluster', key, axis, depth, totalFreq, subNodes:[...], x, y, r }
//   { type:'resource', resource, children:[Resource], totalFreq, x, y, r }
//
// x, y はすべて「ルート座標系の絶対値」で管理する。

// items: [{ r, path }]（path は「選択中の根の子」から葉までの配列）。
// path[level] でグルーピングしながら葉までネストする。
function buildLevel(items, level) {
  // この階層で末端に達した（path をすべて消費した）リソース
  const leaves = items
    .filter(it => it.path.length <= level)
    .map(it => it.r);
  const deeper = items.filter(it => it.path.length > level);

  const groups = d3.group(deeper, it => it.path[level]);
  const clusters = Array.from(groups, ([key, groupItems]) => {
    const totalFreq = groupItems
      .reduce((s, it) => s + (it.r.freq || 1), 0);
    return {
      type: 'cluster',
      key,
      axis: zoomDim,
      depth: level,
      totalFreq,
      subNodes: buildLevel(groupItems, level + 1),
      x: 0, y: 0, r: 0,
    };
  });

  const resourceNodes = leaves
    .map(r => ({
      type: 'resource',
      resource: r,
      children: [],
      totalFreq: r.freq || 1,
      x: 0, y: 0, r: 0,
    }));

  return [...clusters, ...resourceNodes];
}

// ── レイアウト計算（x,y,r を絶対座標で設定） ──────────────────────────────────

// B: あるクラスタのサブツリーで、直下にリソースを最も多く持つクラスタの子数を返す。
// クラスタ半径を「最大密集クラスタに合わせて」全体スケールアップするための計算。
function maxResourceChildCount(node) {
  if (!node.subNodes || node.subNodes.length === 0) return 0;
  if (node.subNodes[0].type === 'resource') return node.subNodes.length;
  return Math.max(...node.subNodes.map(maxResourceChildCount));
}

function layoutTopLevel(clusters) {
  if (clusters.length === 0) return;

  // B: 最も子リソースが集中しているクラスタに合わせてレイアウト全体をスケールする。
  // 3個を「ふつう」の基準にし、それを超えると sqrt でスケールアップ。
  // トップレベルから比率で縮小する階層構造なので、根を広げることで末端まで伝播する。
  const maxLeaves = Math.max(...clusters.map(maxResourceChildCount), 3);
  const leafScale = Math.sqrt(maxLeaves / 3);

  const maxFreq = Math.max(...clusters.map(c => c.totalFreq), 1);
  const maxR = Math.min(W, H) * 0.22 * leafScale;
  const minR = 60 * leafScale;
  const orbitR = Math.min(W, H) * 0.3 * leafScale;

  clusters.forEach((c, i) => {
    c.r = minR + (maxR - minR) * Math.sqrt(c.totalFreq / maxFreq);
    const angle = (i / clusters.length) * 2 * Math.PI - Math.PI / 2;
    c.x = W / 2 + Math.cos(angle) * orbitR;
    c.y = H / 2 + Math.sin(angle) * orbitR;
  });

  runForce(clusters, W / 2, H / 2, null);

  clusters.forEach(c => layoutChildren(c.subNodes, c.x, c.y, c.r));
}

function layoutChildren(nodes, parentX, parentY, parentR) {
  if (!nodes || nodes.length === 0) return;

  if (nodes[0].type === 'resource') {
    layoutResourceNodes(nodes, parentX, parentY, parentR);
    return;
  }

  const maxFreq = Math.max(...nodes.map(c => c.totalFreq), 1);
  const maxR = parentR * 0.40;
  const minR = Math.max(parentR * 0.12, 8);

  nodes.forEach((c, i) => {
    c.r = Math.max(minR, Math.min(maxR, minR + (maxR - minR) * Math.sqrt(c.totalFreq / maxFreq)));
    const angle = (i / Math.max(nodes.length, 1)) * 2 * Math.PI;
    c.x = parentX + Math.cos(angle) * parentR * 0.45;
    c.y = parentY + Math.sin(angle) * parentR * 0.45;
  });

  runForce(nodes, parentX, parentY, parentR);

  nodes.forEach(c => layoutChildren(c.subNodes, c.x, c.y, c.r));
}

function runForce(nodes, cx, cy, boundR) {
  const sim = d3.forceSimulation(nodes)
    .force('center', d3.forceCenter(cx, cy).strength(0.3))
    .force('charge', d3.forceManyBody().strength(-30))
    .force('collide', d3.forceCollide(d => d.r + 5).strength(0.9));

  if (boundR !== null) {
    sim.force('bound', () => {
      nodes.forEach(d => {
        const dx = d.x - cx;
        const dy = d.y - cy;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1e-6;
        const limit = boundR * 0.82 - d.r;
        if (limit > 0 && dist > limit) {
          d.x = cx + (dx / dist) * limit;
          d.y = cy + (dy / dist) * limit;
        }
      });
    });
  }

  sim.stop();
  for (let i = 0; i < 250; i++) sim.tick();
}

function layoutResourceNodes(nodes, parentX, parentY, parentR) {
  const goldenAngle = 137.508 * (Math.PI / 180);
  const spread = parentR * 0.68;

  nodes.forEach((n, i) => {
    const baseR = Math.max(4, Math.min(10, (n.resource.freq || 1) * 0.7 + 2.5));
    n._baseR = baseR;
    if (n.children.length > 0) {
      // 子を内包するための拡張半径: baseR + 子の軌道半径 + 子のサイズ + 余白
      const childOrbit = baseR + 6;
      const childR = 3;
      n.r = childOrbit + childR + 4;
    } else {
      n.r = baseR;
    }
    const angle = i * goldenAngle;
    const dist = Math.min(0.28 * spread * Math.sqrt(i + 1), spread - n.r);
    n.x = parentX + Math.cos(angle) * Math.max(0, dist);
    n.y = parentY + Math.sin(angle) * Math.max(0, dist);
  });

  // A: golden-angle による初期配置の後に collision で重なりを解消する。
  // 初期配置だけでは多数ノードが外縁に密集してクリックできないため。
  runForce(nodes, parentX, parentY, parentR);
}

// ── LOD しきい値 ──────────────────────────────────────────────────────────────
//
// zoomAxes の深さに応じてしきい値を配分する。
// depth 0: 常時表示
// depth N: scale > clusterThreshold(N)
// mini-node, child-node: zoomAxes.length に連動して動的に決まる

function clusterThreshold(depth) {
  // depth 0 → 0, depth 1 → 1.8, depth 2 → 4.5, depth 3 → 7.5 ...
  const steps = [0, 1.8, 4.5, 7.5];
  return steps[depth] ?? steps[steps.length - 1];
}

// mini-node は「最深クラスタの1段先」に相当するしきい値
function lodNodes()      { return clusterThreshold(maxDepth); }
function lodChildren()   { return lodNodes() * 1.7; }
function lodNodeLabel()  { return lodNodes() * 1.3; }
function lodChildLabel() { return lodNodes() * 2.2; }

function clusterLabelOpacity(depth, scale) {
  const show = clusterThreshold(depth);
  const hide = clusterThreshold(depth + 1) || lodNodes();
  if (scale < show) return 0;
  const fadeIn  = Math.min(show + 0.6, (show + hide) / 2);
  const fadeOut = Math.max(hide * 0.75, fadeIn);
  if (scale < fadeIn)  return (scale - show) / (fadeIn - show);
  if (scale < fadeOut) return 1;
  if (scale < hide)    return 1 - (scale - fadeOut) / (hide - fadeOut);
  return 0;
}

function updateLOD(scale) {
  if (!g) return;

  for (let d = 0; d <= maxDepth; d++) {
    const thr = clusterThreshold(d);
    const vis = scale >= thr;
    g.selectAll(`.cluster-d${d}`)
      .style('opacity', vis ? 1 : 0)
      .style('pointer-events', vis ? 'auto' : 'none');
    g.selectAll(`.cluster-label-d${d}, .cluster-count-d${d}`)
      .style('opacity', clusterLabelOpacity(d, scale));
  }

  const nodesVis = scale >= lodNodes();
  g.selectAll('.mini-node')
    .style('opacity', nodesVis ? 1 : 0)
    .style('pointer-events', nodesVis ? 'auto' : 'none');

  g.selectAll('.node-label')
    .style('opacity', scale >= lodNodeLabel() ? 1 : 0);

  const childVis = scale >= lodChildren();
  g.selectAll('.child-node')
    .style('opacity', childVis ? 1 : 0)
    .style('pointer-events', childVis ? 'auto' : 'none');

  g.selectAll('.child-label')
    .style('opacity', scale >= lodChildLabel() ? 1 : 0);
}

// ── レンダリング ──────────────────────────────────────────────────────────────

function render() {
  if (!initialized || !g) return;
  g.selectAll('*').remove();
  if (resources.length === 0) return;

  // リソースを、ズーム軸ディメンションの完全パス付きで集める
  const items = resources
    .map(r => ({ r, path: zoomPath(r) }))
    .filter(it => it.path !== null);

  if (items.length === 0) return;

  maxDepth = Math.max(1, ...items.map(it => it.path.length));

  const tree = buildLevel(items, 0);
  layoutTopLevel(tree);
  drawNodes(g, tree, null);

  updateLOD(currentScale);
  applyTextScale(currentScale);
  updateFilterOpacity();
}

// parentNode: 親クラスタノード（位置計算用）。トップレベルなら null。
function drawNodes(parentG, nodes, parentNode) {
  nodes.forEach(node => {
    // SVG 上の描画座標: parentNode があれば相対変換
    const rx = parentNode ? node.x - parentNode.x : node.x;
    const ry = parentNode ? node.y - parentNode.y : node.y;

    if (node.type === 'cluster') {
      drawCluster(parentG, node, rx, ry);
    } else {
      drawResourceNode(parentG, node, rx, ry);
    }
  });
}

function drawCluster(parentG, cluster, rx, ry) {
  const depth = cluster.depth;
  const color = clusterColor(cluster.key);
  const initOpacity = depth === 0 ? 1 : 0;

  const clusterG = parentG.append('g')
    .attr('class', `cluster cluster-d${depth}`)
    .attr('transform', `translate(${rx},${ry})`)
    .style('opacity', initOpacity);

  // 背景円
  clusterG.append('circle')
    .attr('class', `cluster-bg cluster-bg-d${depth}`)
    .attr('r', cluster.r)
    .attr('fill', color + '22')
    .attr('stroke', color)
    .attr('stroke-width', depth === 0 ? 2 : 1.2)
    .attr('stroke-dasharray', depth > 0 ? '5,3' : null)
    .style('cursor', 'pointer')
    .datum(cluster)
    .on('click', (event, d) => {
      event.stopPropagation();
      // ズームインに合わせて、このクラスタの値を絞り込み軸へ反映（ドリルダウン）。
      // 値なしの「その他」は絞り込み対象にしない。
      if (d.axis && d.key !== 'その他') {
        const cb = window.__cumuloCallbacks.onClusterDrill;
        if (cb) cb(d.axis, d.key);
      }
      zoomToNode(d.x, d.y, d.r);
    });

  // ラベル
  const fs = depth === 0
    ? Math.max(13, cluster.r / 4)
    : Math.max(8, cluster.r / 3.5);

  clusterG.append('text')
    .attr('class', `cluster-label cluster-label-d${depth}`)
    .attr('text-anchor', 'middle')
    .attr('dy', '0.2em')
    .attr('fill', color)
    .attr('data-fs', fs)
    .attr('font-size', fs / currentScale)
    .attr('font-weight', depth === 0 ? 700 : 600)
    .attr('font-family', 'system-ui, sans-serif')
    .attr('pointer-events', 'none')
    .text(nodeLabel(cluster.key));

  const leafCount = countLeaves(cluster);
  const cfs = depth === 0 ? 11 : 9;
  clusterG.append('text')
    .attr('class', `cluster-count cluster-count-d${depth}`)
    .attr('text-anchor', 'middle')
    .attr('dy', fs / 2 + 14)
    .attr('fill', color)
    .attr('fill-opacity', 0.65)
    .attr('data-fs', cfs)
    .attr('font-size', cfs / currentScale)
    .attr('font-family', 'system-ui, sans-serif')
    .attr('pointer-events', 'none')
    .text(`${leafCount} リソース`);

  // サブノードを clusterG 内に相対座標で描画
  drawNodes(clusterG, cluster.subNodes, cluster);
}

function countLeaves(node) {
  if (node.type === 'resource') return 1;
  return (node.subNodes || []).reduce((s, n) => s + countLeaves(n), 0);
}

function drawResourceNode(parentG, node, rx, ry) {
  const r = node.resource;
  const baseR = node._baseR || node.r;
  const hasChildren = node.children.length > 0;

  // ノードカラーは葉（zoomDim の値）で決定
  const color = clusterColor(valueOnAxis(r, zoomDim));

  // クリック時に選択するリソース ID をクロージャで直接キャプチャ
  const resourceId = r.id;

  const nodeG = parentG.append('g')
    .attr('class', hasChildren ? 'mini-node has-children' : 'mini-node')
    .attr('data-id', resourceId)
    .attr('transform', `translate(${rx},${ry})`)
    .style('opacity', 0)
    .style('cursor', 'pointer');

  // リソース本体の円（子がある場合は node.r まで拡張、子はこの円の内側に描画される）
  nodeG.append('circle')
    .attr('class', 'mini-node-circle')
    .attr('r', node.r)
    .attr('fill', color)
    .attr('fill-opacity', filteredIds.has(r.id) ? (hasChildren ? 0.55 : 0.85) : 0.12)
    .attr('stroke', '#0d1117')
    .attr('stroke-width', 1);

  // 名前ラベル: 円の内側中央に表示
  const labelFs = Math.max(5, Math.min(9, node.r * 0.45));
  const maxChars = Math.max(3, Math.floor(node.r * 1.6 / (labelFs * 0.55)));
  const rLabel = resourceLabel(r);
  const labelText = rLabel.length > maxChars ? rLabel.slice(0, maxChars - 1) + '…' : rLabel;
  nodeG.append('text')
    .attr('class', 'node-label')
    .attr('text-anchor', 'middle')
    .attr('dominant-baseline', 'middle')
    .attr('data-fs', labelFs)
    .attr('font-size', labelFs / currentScale)
    .attr('font-family', 'monospace')
    .attr('fill', '#e6edf3')
    .attr('pointer-events', 'none')
    .style('opacity', 0)
    .text(labelText);

  nodeG.on('click', (event) => {
    event.stopPropagation();
    const cb = window.__cumuloCallbacks.onResourceSelect;
    if (cb) cb(resourceId);
  });

  // 子ノード: 親円の内側に配置
  const childOrbit = baseR + 6;  // 内部軌道半径（baseR のすぐ外、container の内側）
  node.children.forEach((child, ci) => {
    const childId = child.id;
    const childAngle = (ci / Math.max(node.children.length, 1)) * 2 * Math.PI;
    const childR = Math.max(2, Math.min(4, (child.freq || 1) * 0.3 + 2));

    const childG = nodeG.append('g')
      .attr('class', 'child-node')
      .attr('data-id', childId)
      .attr('transform', `translate(${Math.cos(childAngle) * childOrbit},${Math.sin(childAngle) * childOrbit})`)
      .style('opacity', 0)
      .style('cursor', 'pointer');

    childG.append('circle')
      .attr('class', 'child-node-circle')
      .attr('r', childR)
      .attr('fill', color)
      .attr('fill-opacity', filteredIds.has(child.id) ? 0.9 : 0.15)
      .attr('stroke', color)
      .attr('stroke-width', 0.8)
      .attr('stroke-opacity', 0.7);

    // 子ノードのラベルも内側中央に
    const childFs = Math.max(4, childR * 0.9);
    childG.append('text')
      .attr('class', 'child-label')
      .attr('text-anchor', 'middle')
      .attr('dominant-baseline', 'middle')
      .attr('data-fs', childFs)
      .attr('font-size', childFs / currentScale)
      .attr('font-family', 'monospace')
      .attr('fill', '#e6edf3')
      .attr('pointer-events', 'none')
      .style('opacity', 0)
      .text(child.name.length > 6 ? child.name.slice(0, 5) + '…' : child.name);

    childG.on('click', (event) => {
      event.stopPropagation();
      const cb = window.__cumuloCallbacks.onResourceSelect;
      if (cb) cb(childId);
    });
  });
}

// ── フィルター透明度更新 ──────────────────────────────────────────────────────

function updateFilterOpacity() {
  if (!g) return;

  g.selectAll('.mini-node').each(function () {
    const el = d3.select(this);
    const id = el.attr('data-id');
    if (!id) return;
    const isParent = el.classed('has-children');
    el.select('.mini-node-circle')
      .transition().duration(250)
      .attr('fill-opacity', filteredIds.has(id) ? (isParent ? 0.55 : 0.85) : 0.1);
  });

  g.selectAll('.child-node').each(function () {
    const el = d3.select(this);
    const id = el.attr('data-id');
    if (!id) return;
    el.select('.child-node-circle')
      .transition().duration(250)
      .attr('fill-opacity', filteredIds.has(id) ? 0.9 : 0.1);
  });

  g.selectAll('.cluster-bg').each(function (d) {
    if (!d) return;
    const allRes = gatherResources(d);
    const hasMatch = allRes.some(r => filteredIds.has(r.id));
    d3.select(this)
      .transition().duration(250)
      .attr('fill', clusterColor(d.key) + (hasMatch ? '28' : '08'))
      .attr('stroke-opacity', hasMatch ? 1 : 0.2);
  });
}

function gatherResources(node) {
  if (!node) return [];
  if (node.type === 'resource') return [node.resource, ...node.children];
  return (node.subNodes || []).flatMap(gatherResources);
}
