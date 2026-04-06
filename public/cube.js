import * as THREE from 'three';

// Face index mapping:
// 0: +Z (front)  → vendor
// 1: +Y (top)    → env
// 2: +X (right)  → category
// 3: -Z (back)   → vendor (opposite)
// 4: -Y (bottom) → env (opposite)
// 5: -X (left)   → category (opposite)

const FACE_NORMALS = [
  new THREE.Vector3(0, 0, 1),   // front  → face 0
  new THREE.Vector3(0, 1, 0),   // top    → face 1
  new THREE.Vector3(1, 0, 0),   // right  → face 2
  new THREE.Vector3(0, 0, -1),  // back   → face 3
  new THREE.Vector3(0, -1, 0),  // bottom → face 4
  new THREE.Vector3(-1, 0, 0),  // left   → face 5
];

const FACE_TARGET_QUATS = [
  new THREE.Quaternion().setFromEuler(new THREE.Euler(0, 0, 0)),                           // front
  new THREE.Quaternion().setFromEuler(new THREE.Euler(-Math.PI / 2, 0, 0)),               // top
  new THREE.Quaternion().setFromEuler(new THREE.Euler(0, -Math.PI / 2, 0)),               // right
  new THREE.Quaternion().setFromEuler(new THREE.Euler(0, Math.PI, 0)),                    // back
  new THREE.Quaternion().setFromEuler(new THREE.Euler(Math.PI / 2, 0, 0)),                // bottom
  new THREE.Quaternion().setFromEuler(new THREE.Euler(0, Math.PI / 2, 0)),                // left
];

// Face label configuration
const FACE_LABELS = ['vendor', 'env', 'category', 'vendor', 'env', 'category'];
const FACE_COLORS = [
  0x185FA5, // AWS blue (front)
  0x2d5016, // env green (top)
  0x6b3fa0, // category purple (right)
  0x0F6E56, // GCP teal (back)
  0x1a3a5c, // env dark (bottom)
  0x3C3489, // Azure purple (left)
];

function createFaceTexture(label, bgColor) {
  const canvas = document.createElement('canvas');
  canvas.width = 256;
  canvas.height = 256;
  const ctx = canvas.getContext('2d');

  // Background
  const r = (bgColor >> 16) & 0xff;
  const g = (bgColor >> 8) & 0xff;
  const b = bgColor & 0xff;
  ctx.fillStyle = `rgb(${r},${g},${b})`;
  ctx.roundRect(8, 8, 240, 240, 24);
  ctx.fill();

  // Border
  ctx.strokeStyle = 'rgba(255,255,255,0.25)';
  ctx.lineWidth = 3;
  ctx.roundRect(8, 8, 240, 240, 24);
  ctx.stroke();

  // Label
  ctx.fillStyle = 'rgba(255,255,255,0.9)';
  ctx.font = 'bold 36px sans-serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, 128, 128);

  return new THREE.CanvasTexture(canvas);
}

class CubeHandle {
  constructor(canvasId) {
    this._faceChangeCallbacks = [];
    this._animating = false;
    this._targetQuat = null;
    this._isDragging = false;
    this._prevMouse = { x: 0, y: 0 };
    this._currentFace = 0;

    const canvas = document.getElementById(canvasId);
    if (!canvas) {
      console.error('Canvas not found:', canvasId);
      return;
    }

    // Scene setup
    this._renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: true,
      alpha: true,
    });
    this._renderer.setPixelRatio(window.devicePixelRatio);
    this._renderer.setSize(canvas.clientWidth || 140, canvas.clientHeight || 140);
    this._renderer.setClearColor(0x000000, 0);

    this._scene = new THREE.Scene();
    this._camera = new THREE.PerspectiveCamera(40, 1, 0.1, 100);
    this._camera.position.set(0, 0, 3.5);

    // Lighting
    const ambient = new THREE.AmbientLight(0xffffff, 0.7);
    this._scene.add(ambient);
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
    dirLight.position.set(2, 3, 4);
    this._scene.add(dirLight);

    // Cube geometry with face textures
    const geometry = new THREE.BoxGeometry(1.8, 1.8, 1.8);
    const materials = FACE_COLORS.map((color, i) =>
      new THREE.MeshLambertMaterial({
        map: createFaceTexture(FACE_LABELS[i], color),
      })
    );
    this._cube = new THREE.Mesh(geometry, materials);
    // Slight initial tilt so all 3 axes are visible
    this._cube.rotation.set(0.4, 0.6, 0);
    this._scene.add(this._cube);

    // Edge highlight
    const edges = new THREE.EdgesGeometry(geometry);
    const lineMat = new THREE.LineBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.15 });
    const wireframe = new THREE.LineSegments(edges, lineMat);
    this._cube.add(wireframe);

    // Drag interaction
    canvas.addEventListener('mousedown', (e) => this._onMouseDown(e));
    canvas.addEventListener('mousemove', (e) => this._onMouseMove(e));
    canvas.addEventListener('mouseup', () => this._onMouseUp());
    canvas.addEventListener('mouseleave', () => this._onMouseUp());

    // Touch
    canvas.addEventListener('touchstart', (e) => {
      const t = e.touches[0];
      this._isDragging = true;
      this._prevMouse = { x: t.clientX, y: t.clientY };
      e.preventDefault();
    }, { passive: false });
    canvas.addEventListener('touchmove', (e) => {
      if (!this._isDragging) return;
      const t = e.touches[0];
      const dx = t.clientX - this._prevMouse.x;
      const dy = t.clientY - this._prevMouse.y;
      this._rotateCubeByDelta(dx, dy);
      this._prevMouse = { x: t.clientX, y: t.clientY };
      e.preventDefault();
    }, { passive: false });
    canvas.addEventListener('touchend', () => {
      this._isDragging = false;
      this._snapToNearestFace();
    });

    this._animate();
  }

  _onMouseDown(e) {
    this._isDragging = true;
    this._prevMouse = { x: e.clientX, y: e.clientY };
    this._targetQuat = null; // cancel any ongoing snap
  }

  _onMouseMove(e) {
    if (!this._isDragging) return;
    const dx = e.clientX - this._prevMouse.x;
    const dy = e.clientY - this._prevMouse.y;
    this._rotateCubeByDelta(dx, dy);
    this._prevMouse = { x: e.clientX, y: e.clientY };
  }

  _onMouseUp() {
    if (!this._isDragging) return;
    this._isDragging = false;
    this._snapToNearestFace();
  }

  _rotateCubeByDelta(dx, dy) {
    const sensitivity = 0.008;
    const qx = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), dx * sensitivity);
    const qy = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), dy * sensitivity);
    this._cube.quaternion.premultiply(qx).premultiply(qy);
  }

  _snapToNearestFace() {
    // Find the face whose normal most aligns with camera direction (0,0,1 in world space)
    const cameraDir = new THREE.Vector3(0, 0, 1);
    let bestFace = 0;
    let bestDot = -Infinity;

    FACE_NORMALS.forEach((normal, i) => {
      const worldNormal = normal.clone().applyQuaternion(this._cube.quaternion);
      const dot = worldNormal.dot(cameraDir);
      if (dot > bestDot) {
        bestDot = dot;
        bestFace = i;
      }
    });

    this._targetQuat = FACE_TARGET_QUATS[bestFace].clone();

    if (bestFace !== this._currentFace) {
      this._currentFace = bestFace;
      this._faceChangeCallbacks.forEach((cb) => cb(bestFace));
    }
  }

  _animate() {
    requestAnimationFrame(() => this._animate());

    if (this._targetQuat && !this._isDragging) {
      this._cube.quaternion.slerp(this._targetQuat, 0.12);
      if (this._cube.quaternion.angleTo(this._targetQuat) < 0.001) {
        this._cube.quaternion.copy(this._targetQuat);
        this._targetQuat = null;
      }
    }

    this._renderer.render(this._scene, this._camera);
  }

  rotateToFace(faceIndex) {
    if (faceIndex < 0 || faceIndex >= FACE_TARGET_QUATS.length) return;
    this._targetQuat = FACE_TARGET_QUATS[faceIndex].clone();
    this._currentFace = faceIndex;
  }

  onFaceChange(callback) {
    this._faceChangeCallbacks.push(callback);
  }

  destroy() {
    this._renderer.dispose();
  }
}

export function initCube(canvasId) {
  return new CubeHandle(canvasId);
}
