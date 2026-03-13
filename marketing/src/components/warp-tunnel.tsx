"use client";

import { useEffect, useRef } from "react";
import * as THREE from "three";

const VERT = `
varying vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

const FRAG = `
uniform float uTime;
uniform vec2 uMouse;
uniform vec2 uResolution;
varying vec2 vUv;

// Simplex-ish noise
vec3 mod289(vec3 x) { return x - floor(x * (1.0/289.0)) * 289.0; }
vec2 mod289(vec2 x) { return x - floor(x * (1.0/289.0)) * 289.0; }
vec3 permute(vec3 x) { return mod289(((x*34.0)+1.0)*x); }

float snoise(vec2 v) {
  const vec4 C = vec4(0.211324865405187, 0.366025403784439,
                     -0.577350269189626, 0.024390243902439);
  vec2 i  = floor(v + dot(v, C.yy));
  vec2 x0 = v -   i + dot(i, C.xx);
  vec2 i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
  vec4 x12 = x0.xyxy + C.xxzz;
  x12.xy -= i1;
  i = mod289(i);
  vec3 p = permute(permute(i.y + vec3(0.0, i1.y, 1.0)) + i.x + vec3(0.0, i1.x, 1.0));
  vec3 m = max(0.5 - vec3(dot(x0,x0), dot(x12.xy,x12.xy), dot(x12.zw,x12.zw)), 0.0);
  m = m*m; m = m*m;
  vec3 x = 2.0 * fract(p * C.www) - 1.0;
  vec3 h = abs(x) - 0.5;
  vec3 a0 = x - floor(x + 0.5);
  m *= 1.79284291400159 - 0.85373472095314 * (a0*a0 + h*h);
  vec3 g;
  g.x = a0.x * x0.x + h.x * x0.y;
  g.yz = a0.yz * x12.xz + h.yz * x12.yw;
  return 130.0 * dot(m, g);
}

void main() {
  vec2 uv = vUv;
  vec2 center = vec2(0.5);

  // Mouse influence — subtle offset
  vec2 mouse = uMouse * 0.03;
  center += mouse;

  vec2 p = uv - center;
  float aspect = uResolution.x / uResolution.y;
  p.x *= aspect;

  // Polar coordinates
  float r = length(p);
  float angle = atan(p.y, p.x);

  // Tunnel depth — inverse of radius creates the infinite tunnel illusion
  float depth = 0.4 / (r + 0.001);

  // Scroll through the tunnel
  float speed = uTime * 0.6;

  // Warp the tunnel coordinates with noise
  float warp1 = snoise(vec2(angle * 2.0 + uTime * 0.15, depth * 0.3 + speed)) * 0.3;
  float warp2 = snoise(vec2(angle * 3.0 - uTime * 0.1, depth * 0.5 - speed * 0.7)) * 0.15;

  // Tunnel texture — rings and spirals
  float rings = sin(depth * 4.0 + speed + warp1) * 0.5 + 0.5;
  float spirals = sin(angle * 6.0 + depth * 2.0 + uTime * 0.3 + warp2) * 0.5 + 0.5;

  // Energy lines flowing through the tunnel
  float lines = smoothstep(0.85, 1.0, sin(angle * 12.0 + depth * 3.0 + speed * 2.0 + warp1 * 2.0));

  // Combine patterns
  float pattern = rings * 0.4 + spirals * 0.3 + lines * 0.5;

  // Color palette — teal core, purple edges, dark blue depth
  vec3 teal = vec3(0.176, 0.831, 0.749);    // #2dd4bf
  vec3 purple = vec3(0.655, 0.545, 0.980);  // #a78bfa
  vec3 blue = vec3(0.376, 0.510, 0.976);    // #6082f9
  vec3 dark = vec3(0.035, 0.035, 0.043);    // #09090b

  // Color mixing based on depth and angle
  float colorMix = sin(depth * 0.5 + uTime * 0.2) * 0.5 + 0.5;
  vec3 tunnelColor = mix(teal, purple, colorMix);
  tunnelColor = mix(tunnelColor, blue, spirals * 0.3);

  // Apply pattern intensity
  float intensity = pattern * smoothstep(0.0, 0.15, r) * smoothstep(1.5, 0.1, r);

  // Add glow at the center (the "portal" opening)
  float centerGlow = exp(-r * 8.0) * (0.5 + 0.3 * sin(uTime * 0.8));

  // Vignette — fade to dark at edges
  float vignette = 1.0 - smoothstep(0.3, 0.85, r);

  // Final color
  vec3 color = dark;
  color += tunnelColor * intensity * 0.35 * vignette;
  color += teal * centerGlow * 0.6;
  color += tunnelColor * lines * 0.15 * vignette;

  // Subtle edge glow
  float edgeNoise = snoise(vec2(angle * 4.0 + uTime * 0.1, r * 3.0)) * 0.5 + 0.5;
  color += teal * edgeNoise * 0.02 * vignette;

  // Keep it dark overall — this is a background
  color *= 0.7;

  gl_FragColor = vec4(color, 1.0);
}
`;

export function WarpTunnel() {
  const containerRef = useRef<HTMLDivElement>(null);
  const mouseRef = useRef({ x: 0, y: 0 });
  const smoothMouse = useRef({ x: 0, y: 0 });

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const scene = new THREE.Scene();
    const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);

    const renderer = new THREE.WebGLRenderer({
      antialias: false,
      alpha: false,
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(container.clientWidth, container.clientHeight);
    container.appendChild(renderer.domElement);

    const uniforms = {
      uTime: { value: 0 },
      uMouse: { value: new THREE.Vector2(0, 0) },
      uResolution: { value: new THREE.Vector2(container.clientWidth, container.clientHeight) },
    };

    const material = new THREE.ShaderMaterial({
      vertexShader: VERT,
      fragmentShader: FRAG,
      uniforms,
    });

    const mesh = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), material);
    scene.add(mesh);

    // Mouse tracking
    const onMouseMove = (e: MouseEvent) => {
      mouseRef.current.x = (e.clientX / window.innerWidth) * 2 - 1;
      mouseRef.current.y = -(e.clientY / window.innerHeight) * 2 + 1;
    };
    window.addEventListener("mousemove", onMouseMove);

    // Resize
    const onResize = () => {
      const w = container.clientWidth;
      const h = container.clientHeight;
      renderer.setSize(w, h);
      uniforms.uResolution.value.set(w, h);
    };
    window.addEventListener("resize", onResize);

    // Animation loop
    let frameId: number;
    const clock = new THREE.Clock();

    const animate = () => {
      frameId = requestAnimationFrame(animate);

      uniforms.uTime.value = clock.getElapsedTime();

      // Smooth mouse interpolation
      smoothMouse.current.x += (mouseRef.current.x - smoothMouse.current.x) * 0.05;
      smoothMouse.current.y += (mouseRef.current.y - smoothMouse.current.y) * 0.05;
      uniforms.uMouse.value.set(smoothMouse.current.x, smoothMouse.current.y);

      renderer.render(scene, camera);
    };
    animate();

    return () => {
      cancelAnimationFrame(frameId);
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("resize", onResize);
      renderer.dispose();
      material.dispose();
      container.removeChild(renderer.domElement);
    };
  }, []);

  return (
    <div
      ref={containerRef}
      className="absolute inset-0 z-0"
      style={{
        opacity: 0.6,
        maskImage: "linear-gradient(to bottom, black 40%, transparent 100%)",
        WebkitMaskImage: "linear-gradient(to bottom, black 40%, transparent 100%)",
      }}
    />
  );
}
