import * as THREE from "https://cdn.jsdelivr.net/npm/three@0.160.0/build/three.module.js";
import init, { Simulator } from "./pkg/quadcopter_web_sim.js?v=5";

const WALL_TEXTURE_URL =
  "https://grizly.club/uploads/posts/2023-01/1672825427_grizly-club-p-tekstura-khrushchevki-5.jpg";
const ROOF_TEXTURE_URL =
  "https://img.freepik.com/free-vector/gradient-roof-tile-pattern-design_23-2149264068.jpg?semt=ais_rp_progressive&w=740&q=80";

const CONTROL_KEYS = new Set([
  "KeyW",
  "KeyS",
  "KeyA",
  "KeyD",
  "KeyQ",
  "KeyE",
  "KeyR",
  "KeyF",
  "KeyP",
  "Space",
]);

const canvas = document.getElementById("viewport");
const loading = document.getElementById("loading");

let renderer;
let scene;
let camera;
let simulator;
let drone;
let propellers = [];
let lastTime = 0;

const tmpQuatYaw = new THREE.Quaternion();
const tmpQuatPitch = new THREE.Quaternion();
const tmpQuatRoll = new THREE.Quaternion();
const yawAxis = new THREE.Vector3(0, 1, 0);
const pitchAxis = new THREE.Vector3(1, 0, 0);
const rollAxis = new THREE.Vector3(0, 0, 1);
const cameraDesired = new THREE.Vector3();
const cameraTarget = new THREE.Vector3();
const velocityVector = new THREE.Vector3();
const dronePos = new THREE.Vector3();

boot().catch((error) => {
  console.error(error);
  loading.remove();
  const box = document.createElement("div");
  box.className = "error-box";
  box.innerHTML = `
    <strong>Не удалось загрузить симулятор.</strong><br />
    Сначала выполни <code>wasm-pack build --release --target web --out-dir docs/pkg</code>,<br />
    затем открой папку <code>docs</code> через локальный сервер или GitHub Pages.
  `;
  document.body.appendChild(box);
});

async function boot() {
  await init();
  simulator = new Simulator();

  setupRenderer();
  scene = new THREE.Scene();
  scene.background = new THREE.Color(0xaecff3);
  scene.fog = new THREE.Fog(0xaecff3, 45, 170);

  camera = new THREE.PerspectiveCamera(58, window.innerWidth / window.innerHeight, 0.1, 260);
  camera.position.set(-9, 5, -10);

  addLights();
  addGround();
  addRoads();

  const [wallTexture, roofTexture] = await Promise.all([
    loadTextureWithFallback(WALL_TEXTURE_URL, createWallFallbackTexture),
    loadTextureWithFallback(ROOF_TEXTURE_URL, createRoofFallbackTexture),
  ]);

  addBuildings(wallTexture, roofTexture);
  drone = createDrone();
  scene.add(drone.group);

  setupInput();
  window.addEventListener("resize", onResize);
  onResize();

  loading.remove();
  requestAnimationFrame(frame);
}

function setupRenderer() {
  renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
}

function addLights() {
  const hemi = new THREE.HemisphereLight(0xd8ebff, 0x5a7749, 1.15);
  hemi.position.set(0, 80, 0);
  scene.add(hemi);

  const sun = new THREE.DirectionalLight(0xffffff, 1.35);
  sun.position.set(18, 28, 12);
  sun.castShadow = true;
  sun.shadow.mapSize.width = 2048;
  sun.shadow.mapSize.height = 2048;
  sun.shadow.camera.near = 0.5;
  sun.shadow.camera.far = 90;
  sun.shadow.camera.left = -38;
  sun.shadow.camera.right = 38;
  sun.shadow.camera.top = 38;
  sun.shadow.camera.bottom = -38;
  scene.add(sun);
}

function addGround() {
  const ground = new THREE.Mesh(
    new THREE.PlaneGeometry(260, 260),
    new THREE.MeshStandardMaterial({ color: 0x4d8d42, roughness: 0.98, metalness: 0.02 }),
  );
  ground.rotation.x = -Math.PI / 2;
  ground.receiveShadow = true;
  scene.add(ground);

  for (const [x, z, w, h, color] of [
    [-48, -42, 40, 24, 0x588f4e],
    [52, -35, 34, 18, 0x467a3e],
    [-56, 36, 28, 20, 0x5d924f],
    [48, 42, 36, 22, 0x548748],
  ]) {
    const patch = new THREE.Mesh(
      new THREE.PlaneGeometry(w, h),
      new THREE.MeshStandardMaterial({ color, roughness: 1, metalness: 0 }),
    );
    patch.rotation.x = -Math.PI / 2;
    patch.position.set(x, 0.01, z);
    patch.receiveShadow = true;
    scene.add(patch);
  }
}

function addRoads() {
  const roadMaterial = new THREE.MeshStandardMaterial({ color: 0x565656, roughness: 0.95, metalness: 0.03 });
  const sidewalkMaterial = new THREE.MeshStandardMaterial({ color: 0xa9a9a9, roughness: 0.95, metalness: 0.02 });

  const roadA = new THREE.Mesh(new THREE.BoxGeometry(16, 0.02, 240), roadMaterial);
  roadA.position.set(0, 0.02, 0);
  roadA.receiveShadow = true;
  scene.add(roadA);

  const roadB = new THREE.Mesh(new THREE.BoxGeometry(240, 0.02, 16), roadMaterial);
  roadB.position.set(0, 0.021, 0);
  roadB.receiveShadow = true;
  scene.add(roadB);

  for (const [x, z, w, d] of [
    [-12, 0, 4, 240],
    [12, 0, 4, 240],
    [0, -12, 240, 4],
    [0, 12, 240, 4],
  ]) {
    const sidewalk = new THREE.Mesh(new THREE.BoxGeometry(w, 0.03, d), sidewalkMaterial);
    sidewalk.position.set(x, 0.035, z);
    sidewalk.receiveShadow = true;
    scene.add(sidewalk);
  }
}

function addBuildings(wallTexture, roofTexture) {
  const layouts = [
    { x: -34, z: -34, w: 13, d: 8, h: 16 },
    { x: -34, z: 34, w: 12, d: 8, h: 15 },
    { x: 34, z: -34, w: 13, d: 8, h: 17 },
    { x: 34, z: 34, w: 12, d: 8, h: 16 },
    { x: -58, z: -14, w: 11, d: 8, h: 14 },
    { x: -58, z: 14, w: 11, d: 8, h: 14 },
    { x: 58, z: -14, w: 11, d: 8, h: 14 },
    { x: 58, z: 14, w: 11, d: 8, h: 14 },
    { x: -14, z: -58, w: 8, d: 12, h: 15 },
    { x: 14, z: -58, w: 8, d: 12, h: 15 },
    { x: -14, z: 58, w: 8, d: 12, h: 15 },
    { x: 14, z: 58, w: 8, d: 12, h: 15 },
  ];

  for (const building of layouts) {
    const group = new THREE.Group();

    const sideTextureA = stretchTexture(wallTexture);
    const sideTextureB = stretchTexture(wallTexture);
    const roofTextureClone = cloneTexture(roofTexture, Math.max(2, Math.round(building.w / 2.4)), Math.max(2, Math.round(building.d / 2.4)));

    const facadeMaterials = [
      new THREE.MeshStandardMaterial({ map: sideTextureB, roughness: 0.96, metalness: 0.02 }),
      new THREE.MeshStandardMaterial({ map: sideTextureB, roughness: 0.96, metalness: 0.02 }),
      new THREE.MeshStandardMaterial({ map: roofTextureClone, roughness: 0.94, metalness: 0.03 }),
      new THREE.MeshStandardMaterial({ color: 0x8c8d89, roughness: 1, metalness: 0 }),
      new THREE.MeshStandardMaterial({ map: sideTextureA, roughness: 0.96, metalness: 0.02 }),
      new THREE.MeshStandardMaterial({ map: sideTextureA, roughness: 0.96, metalness: 0.02 }),
    ];

    const body = new THREE.Mesh(new THREE.BoxGeometry(building.w, building.h, building.d), facadeMaterials);
    body.position.y = building.h * 0.5;
    body.castShadow = true;
    body.receiveShadow = true;
    group.add(body);

    const entrance = new THREE.Mesh(
      new THREE.BoxGeometry(Math.min(2.4, building.w * 0.32), 2.5, 1.3),
      new THREE.MeshStandardMaterial({ color: 0xa39c90, roughness: 0.98, metalness: 0.02 }),
    );
    entrance.position.set(0, 1.25, building.d * 0.5 + 0.65);
    entrance.castShadow = true;
    entrance.receiveShadow = true;
    group.add(entrance);

    const canopy = new THREE.Mesh(
      new THREE.BoxGeometry(Math.min(2.8, building.w * 0.36), 0.12, 1.5),
      new THREE.MeshStandardMaterial({ color: 0x6c6c70, roughness: 0.7, metalness: 0.2 }),
    );
    canopy.position.set(0, 2.45, building.d * 0.5 + 0.3);
    canopy.castShadow = true;
    canopy.receiveShadow = true;
    group.add(canopy);

    group.position.set(building.x, 0, building.z);
    scene.add(group);
  }
}

function createDrone() {
  const group = new THREE.Group();
  const bodyMaterial = new THREE.MeshStandardMaterial({ color: 0x8b9096, roughness: 0.7, metalness: 0.28 });
  const darkMaterial = new THREE.MeshStandardMaterial({ color: 0x50535a, roughness: 0.65, metalness: 0.3 });
  const propMaterial = new THREE.MeshStandardMaterial({ color: 0x6b6f75, roughness: 0.82, metalness: 0.1, transparent: true, opacity: 0.7 });

  const coreBase = new THREE.Mesh(new THREE.BoxGeometry(0.78, 0.12, 0.46), darkMaterial);
  coreBase.castShadow = true;
  group.add(coreBase);

  const coreTop = new THREE.Mesh(new THREE.BoxGeometry(0.52, 0.18, 0.34), bodyMaterial);
  coreTop.position.y = 0.12;
  coreTop.castShadow = true;
  group.add(coreTop);

  const battery = new THREE.Mesh(new THREE.BoxGeometry(0.26, 0.1, 0.18), new THREE.MeshStandardMaterial({ color: 0x70757d, roughness: 0.55, metalness: 0.4 }));
  battery.position.set(0, 0.22, -0.02);
  battery.castShadow = true;
  group.add(battery);

  const armGeom = new THREE.BoxGeometry(2.1, 0.06, 0.1);
  const armA = new THREE.Mesh(armGeom, bodyMaterial);
  armA.rotation.y = Math.PI / 4;
  armA.castShadow = true;
  group.add(armA);

  const armB = new THREE.Mesh(armGeom, bodyMaterial);
  armB.rotation.y = -Math.PI / 4;
  armB.castShadow = true;
  group.add(armB);

  const motorPositions = [
    [-0.78, 0.02, 0.78],
    [0.78, 0.02, 0.78],
    [-0.78, 0.02, -0.78],
    [0.78, 0.02, -0.78],
  ];

  const motorGeom = new THREE.CylinderGeometry(0.11, 0.11, 0.12, 20);
  const propGeom = new THREE.CylinderGeometry(0.33, 0.33, 0.016, 28);
  const skidGeom = new THREE.CylinderGeometry(0.03, 0.03, 1.25, 12);

  propellers = [];
  for (const [index, position] of motorPositions.entries()) {
    const motor = new THREE.Mesh(motorGeom, darkMaterial);
    motor.rotation.x = Math.PI / 2;
    motor.position.set(...position);
    motor.castShadow = true;
    group.add(motor);

    const prop = new THREE.Mesh(propGeom, propMaterial);
    prop.rotation.x = Math.PI / 2;
    prop.position.set(position[0], position[1] + 0.07, position[2]);
    prop.castShadow = true;
    group.add(prop);
    propellers.push({ mesh: prop, direction: index % 2 === 0 ? 1 : -1 });
  }

  const skidLeft = new THREE.Mesh(skidGeom, darkMaterial);
  skidLeft.rotation.z = Math.PI / 2;
  skidLeft.position.set(0, -0.22, 0.26);
  skidLeft.castShadow = true;
  group.add(skidLeft);

  const skidRight = new THREE.Mesh(skidGeom, darkMaterial);
  skidRight.rotation.z = Math.PI / 2;
  skidRight.position.set(0, -0.22, -0.26);
  skidRight.castShadow = true;
  group.add(skidRight);

  for (const side of [-1, 1]) {
    for (const z of [-0.26, 0.26]) {
      const leg = new THREE.Mesh(new THREE.CylinderGeometry(0.02, 0.02, 0.22, 10), darkMaterial);
      leg.position.set(side * 0.42, -0.13, z);
      leg.rotation.z = side * 0.22;
      leg.castShadow = true;
      group.add(leg);
    }
  }

  return { group };
}

function setupInput() {
  window.addEventListener("keydown", (event) => {
    if (!CONTROL_KEYS.has(event.code)) {
      return;
    }
    event.preventDefault();
    if (event.repeat) {
      return;
    }
    simulator.set_key(event.code, true);
  });

  window.addEventListener("keyup", (event) => {
    if (!CONTROL_KEYS.has(event.code)) {
      return;
    }
    event.preventDefault();
    simulator.set_key(event.code, false);
  });
}

function onResize() {
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
}

function frame(timestamp) {
  const dt = Math.min((timestamp - lastTime) / 1000 || 0.016, 0.05);
  lastTime = timestamp;

  simulator.update(dt);
  updateDroneMesh(dt);

  renderer.render(scene, camera);
  requestAnimationFrame(frame);
}

function updateDroneMesh(dt) {
  dronePos.set(simulator.x(), simulator.y(), simulator.z());
  drone.group.position.copy(dronePos);

  tmpQuatYaw.setFromAxisAngle(yawAxis, simulator.yaw());
  tmpQuatPitch.setFromAxisAngle(pitchAxis, simulator.pitch());
  tmpQuatRoll.setFromAxisAngle(rollAxis, simulator.roll());
  drone.group.quaternion.copy(tmpQuatYaw).multiply(tmpQuatPitch).multiply(tmpQuatRoll);

  for (let i = 0; i < propellers.length; i += 1) {
    const thrust = simulator.motor(i);
    propellers[i].mesh.rotation.y += (18 + thrust * 52) * dt * propellers[i].direction;
  }

  velocityVector.set(simulator.velocity_x(), simulator.velocity_y(), simulator.velocity_z());
  const behind = new THREE.Vector3(0, 3.5, -11).applyAxisAngle(yawAxis, simulator.yaw());
  cameraDesired.copy(dronePos).add(behind);
  camera.position.lerp(cameraDesired, 1 - Math.exp(-3.2 * dt));

  cameraTarget.copy(dronePos).add(new THREE.Vector3(0, 0.95, 0)).addScaledVector(velocityVector, 0.16);
  camera.lookAt(cameraTarget);
}

async function loadTextureWithFallback(url, fallbackFactory) {
  const loader = new THREE.TextureLoader();
  loader.setCrossOrigin("anonymous");

  try {
    const texture = await new Promise((resolve, reject) => {
      loader.load(url, resolve, undefined, reject);
    });
    return tuneTexture(texture);
  } catch (error) {
    console.warn(`Texture fallback used for ${url}`, error);
    return tuneTexture(fallbackFactory());
  }
}

function tuneTexture(texture) {
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = Math.min(renderer.capabilities.getMaxAnisotropy(), 8);
  texture.needsUpdate = true;
  return texture;
}

function cloneTexture(source, repeatX, repeatY) {
  const texture = source.clone();
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.repeat.set(repeatX, repeatY);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = source.anisotropy;
  texture.needsUpdate = true;
  return texture;
}

function stretchTexture(source) {
  const texture = source.clone();
  texture.wrapS = THREE.ClampToEdgeWrapping;
  texture.wrapT = THREE.ClampToEdgeWrapping;
  texture.repeat.set(1, 1);
  texture.offset.set(0, 0);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = source.anisotropy;
  texture.needsUpdate = true;
  return texture;
}

function createWallFallbackTexture() {
  const canvas = document.createElement("canvas");
  canvas.width = 512;
  canvas.height = 512;
  const ctx = canvas.getContext("2d");

  ctx.fillStyle = "#c7c1b7";
  ctx.fillRect(0, 0, 512, 512);

  ctx.fillStyle = "#b3aea6";
  for (let y = 0; y < 512; y += 128) {
    ctx.fillRect(0, y, 512, 8);
  }

  for (let row = 0; row < 5; row += 1) {
    for (let col = 0; col < 6; col += 1) {
      const x = 36 + col * 76;
      const y = 32 + row * 90;
      ctx.fillStyle = "#9aa7b1";
      ctx.fillRect(x, y, 38, 44);
      ctx.fillStyle = "rgba(245, 235, 180, 0.45)";
      ctx.fillRect(x + 4, y + 4, 14, 14);
      ctx.fillStyle = "rgba(46, 56, 68, 0.35)";
      ctx.fillRect(x + 20, y + 4, 14, 14);
      ctx.fillStyle = "#d4d0c7";
      ctx.fillRect(x, y + 48, 38, 8);
    }
  }

  ctx.fillStyle = "#8c8479";
  ctx.fillRect(222, 432, 66, 80);
  ctx.fillStyle = "#5c4e41";
  ctx.fillRect(244, 446, 22, 66);

  return new THREE.CanvasTexture(canvas);
}

function createRoofFallbackTexture() {
  const canvas = document.createElement("canvas");
  canvas.width = 512;
  canvas.height = 512;
  const ctx = canvas.getContext("2d");

  const gradient = ctx.createLinearGradient(0, 0, 0, 512);
  gradient.addColorStop(0, "#8f5350");
  gradient.addColorStop(1, "#5f3330");
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 512, 512);

  ctx.strokeStyle = "rgba(255, 255, 255, 0.16)";
  ctx.lineWidth = 6;
  for (let y = 0; y < 512; y += 34) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    for (let x = 0; x <= 512; x += 32) {
      ctx.quadraticCurveTo(x + 16, y + 10, x + 32, y);
    }
    ctx.stroke();
  }

  ctx.strokeStyle = "rgba(30, 12, 12, 0.18)";
  ctx.lineWidth = 2;
  for (let y = 18; y < 512; y += 34) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    for (let x = 0; x <= 512; x += 32) {
      ctx.quadraticCurveTo(x + 16, y + 8, x + 32, y);
    }
    ctx.stroke();
  }

  return new THREE.CanvasTexture(canvas);
}
