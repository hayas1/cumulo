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
    if (event.target === svg.node()) zoomToFit();
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
    if (initialized) { render(); zoomToFit(); }
  } catch (e) { console.error('cumuloUpdateZoomAxes', e); }
};

window.cumuloUpdateDimensions = function (json) {
  try {
    const dimensions = JSON.parse(json);
    valueColors = {};
    dimensions.forEach(dim => {
      (dim.values || []).forEach(dv => {
        if (dv.color) valueColors[dv.value] = dv.color;
      });
    });
    if (initialized) render();
  } catch (e) { console.error('cumuloUpdateDimensions', e); }
};

window.cumuloZoomToFit = function () { zoomToFit(); };

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
}

// ── ズーム遷移 ────────────────────────────────────────────────────────────────

// absX, absY はルート座標系での絶対位置
function zoomToNode(absX, absY, r) {
  // 自然なズームスケール（クラスタが画面の85%を占める）と
  // mini-node が見えるための最低スケールのうち大きい方を使う
  const naturalScale = (Math.min(W, H) * 0.85) / (r * 2);
  const scale = Math.max(naturalScale, lodNodes() + 0.3);
  const tx = W / 2 - scale * absX;
  const ty = H / 2 - scale * absY;
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

// ── 階層ツリー構築 ────────────────────────────────────────────────────────────
//
// ノード型:
//   { type:'cluster', key, axis, depth, totalFreq, subNodes:[...], x, y, r }
//   { type:'resource', resource, children:[Resource], totalFreq, x, y, r }
//
// x, y はすべて「ルート座標系の絶対値」で管理する。

function buildTree(items, axes, depth) {
  if (axes.length === 0) {
    // 末端: root リソースのみ（children は resource ノード内に保持）
    return items
      .filter(r => !r.parent_id)
      .map(r => ({
        type: 'resource',
        resource: r,
        children: resources.filter(c => c.parent_id === r.id),
        totalFreq: r.freq || 1,
        x: 0, y: 0, r: 0,
      }));
  }

  const axis = axes[0];
  const groups = d3.group(items, r => (r.attrs && r.attrs[axis]) || 'その他');

  return Array.from(groups, ([key, groupItems]) => {
    const totalFreq = groupItems
      .filter(r => !r.parent_id)   // freq は root リソースだけ積算
      .reduce((s, r) => s + (r.freq || 1), 0);
    return {
      type: 'cluster',
      key,
      axis,
      depth,
      totalFreq,
      subNodes: buildTree(groupItems, axes.slice(1), depth + 1),
      x: 0, y: 0, r: 0,
    };
  });
}

// ── レイアウト計算（x,y,r を絶対座標で設定） ──────────────────────────────────

function layoutTopLevel(clusters) {
  if (clusters.length === 0) return;

  const maxFreq = Math.max(...clusters.map(c => c.totalFreq), 1);
  const maxR = Math.min(W, H) * 0.22;
  const minR = 60;

  clusters.forEach((c, i) => {
    c.r = minR + (maxR - minR) * Math.sqrt(c.totalFreq / maxFreq);
    const angle = (i / clusters.length) * 2 * Math.PI - Math.PI / 2;
    c.x = W / 2 + Math.cos(angle) * Math.min(W, H) * 0.3;
    c.y = H / 2 + Math.sin(angle) * Math.min(W, H) * 0.3;
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
function lodNodes()      { return clusterThreshold(zoomAxes.length); }
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

  for (let d = 0; d <= zoomAxes.length; d++) {
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

  const tree = buildTree(resources, zoomAxes, 0);
  layoutTopLevel(tree);
  drawNodes(g, tree, null);

  updateLOD(currentScale);
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
    .attr('font-size', fs)
    .attr('font-weight', depth === 0 ? 700 : 600)
    .attr('font-family', 'system-ui, sans-serif')
    .attr('pointer-events', 'none')
    .text(cluster.key);

  const leafCount = countLeaves(cluster);
  clusterG.append('text')
    .attr('class', `cluster-count cluster-count-d${depth}`)
    .attr('text-anchor', 'middle')
    .attr('dy', fs / 2 + 14)
    .attr('fill', color)
    .attr('fill-opacity', 0.65)
    .attr('font-size', depth === 0 ? 11 : 9)
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

  // 最終ズーム軸の値でノードカラーを決定
  const lastAxis = zoomAxes[zoomAxes.length - 1];
  const color = clusterColor(r.attrs && r.attrs[lastAxis]);

  const nodeG = parentG.append('g')
    .attr('class', hasChildren ? 'mini-node has-children' : 'mini-node')
    .attr('transform', `translate(${rx},${ry})`)
    .style('opacity', 0)
    .style('cursor', 'pointer')
    .datum(r);

  // リソース本体の円（子がある場合は node.r まで拡張、子はこの円の内側に描画される）
  nodeG.append('circle')
    .attr('class', 'mini-node-circle')
    .attr('r', node.r)
    .attr('fill', color)
    .attr('fill-opacity', filteredIds.has(r.id) ? (hasChildren ? 0.55 : 0.85) : 0.12)
    .attr('stroke', '#0d1117')
    .attr('stroke-width', 1);

  nodeG.append('text')
    .attr('class', 'node-label')
    .attr('text-anchor', 'middle')
    .attr('dy', node.r + 11)
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

  // 子ノード: 親円の内側に配置
  const childOrbit = baseR + 6;  // 内部軌道半径（baseR のすぐ外、container の内側）
  node.children.forEach((child, ci) => {
    const childAngle = (ci / Math.max(node.children.length, 1)) * 2 * Math.PI;
    const childR = Math.max(2, Math.min(4, (child.freq || 1) * 0.3 + 2));

    const childG = nodeG.append('g')
      .attr('class', 'child-node')
      .attr('transform', `translate(${Math.cos(childAngle) * childOrbit},${Math.sin(childAngle) * childOrbit})`)
      .style('opacity', 0)
      .style('cursor', 'pointer')
      .datum(child);

    childG.append('circle')
      .attr('class', 'child-node-circle')
      .attr('r', childR)
      .attr('fill', color)
      .attr('fill-opacity', filteredIds.has(child.id) ? 0.9 : 0.15)
      .attr('stroke', color)
      .attr('stroke-width', 0.8)
      .attr('stroke-opacity', 0.7);

    childG.append('text')
      .attr('class', 'child-label')
      .attr('text-anchor', 'middle')
      .attr('dy', childR + 8)
      .attr('font-size', 7)
      .attr('font-family', 'monospace')
      .attr('fill', '#8b949e')
      .attr('pointer-events', 'none')
      .style('opacity', 0)
      .text(child.name);

    childG.on('click', (event, resource) => {
      event.stopPropagation();
      const cb = window.__cumuloCallbacks.onResourceSelect;
      if (cb) cb(resource.id);
    });
  });
}

// ── フィルター透明度更新 ──────────────────────────────────────────────────────

function updateFilterOpacity() {
  if (!g) return;

  g.selectAll('.mini-node').each(function (d) {
    if (!d) return;
    const isParent = d3.select(this).classed('has-children');
    d3.select(this).select('.mini-node-circle')
      .transition().duration(250)
      .attr('fill-opacity', filteredIds.has(d.id) ? (isParent ? 0.55 : 0.85) : 0.1);
  });

  g.selectAll('.child-node').each(function (d) {
    if (!d) return;
    d3.select(this).select('.child-node-circle')
      .transition().duration(250)
      .attr('fill-opacity', filteredIds.has(d.id) ? 0.9 : 0.1);
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
