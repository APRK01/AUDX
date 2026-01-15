import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

const NUM_BARS = 64;

function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const barsRef = useRef<number[]>(new Array(NUM_BARS).fill(0));
  const targetRef = useRef<number[]>(new Array(NUM_BARS).fill(0));
  const frameRef = useRef<number>(0);

  useEffect(() => {
    invoke("start_audio_listener").catch(console.error);

    const unlisten = listen<number[]>("audio-data", (e) => {
      targetRef.current = e.payload;
    });

    const canvas = canvasRef.current!;
    const ctx = canvas.getContext("2d")!;

    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      const rect = canvas.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      ctx.scale(dpr, dpr);
    };

    resize();
    window.addEventListener("resize", resize);

    const draw = () => {
      const rect = canvas.getBoundingClientRect();
      const w = rect.width;
      const h = rect.height;

      ctx.clearRect(0, 0, w, h);

      const gap = 2;
      const totalWidth = w - gap * (NUM_BARS + 1);
      const barW = totalWidth / NUM_BARS;

      for (let i = 0; i < NUM_BARS; i++) {
        const target = targetRef.current[i] || 0;
        const current = barsRef.current[i];

        if (target > current) {
          barsRef.current[i] = current + (target - current) * 0.3;
        } else {
          barsRef.current[i] = current + (target - current) * 0.1;
        }

        const value = Math.min(barsRef.current[i], 1);
        const barH = Math.max(value * h * 0.9, 2);

        const x = gap + i * (barW + gap);
        const y = h - barH;

        const hue = 180 + i * 1.5;
        const sat = 70 + value * 30;
        const light = 50 + value * 20;

        const grad = ctx.createLinearGradient(x, h, x, y);
        grad.addColorStop(0, `hsla(${hue}, ${sat}%, ${light - 10}%, 0.8)`);
        grad.addColorStop(0.5, `hsla(${hue}, ${sat}%, ${light}%, 0.9)`);
        grad.addColorStop(1, `hsla(${hue + 30}, ${sat}%, ${light + 15}%, 1)`);

        ctx.fillStyle = grad;

        const r = Math.min(barW / 2, 4);
        ctx.beginPath();
        ctx.roundRect(x, y, barW, barH, [r, r, 0, 0]);
        ctx.fill();
      }

      frameRef.current = requestAnimationFrame(draw);
    };

    draw();

    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("resize", resize);
      cancelAnimationFrame(frameRef.current);
    };
  }, []);

  return (
    <div className="app">
      <div className="drag-handle" data-tauri-drag-region />
      <canvas ref={canvasRef} className="canvas" />
    </div>
  );
}

export default App;
