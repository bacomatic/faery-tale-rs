import { useEffect, useState } from "react";
import TaskList from "./pages/TaskList";
import TaskPage from "./pages/TaskPage";

function useHash() {
  const [hash, setHash] = useState(location.hash);
  useEffect(() => {
    const onChange = () => setHash(location.hash);
    addEventListener("hashchange", onChange);
    return () => removeEventListener("hashchange", onChange);
  }, []);
  return hash;
}

const BACKDROPS: Record<string, [string, string]> = {
  grey: ["#2a2d34", "#22252b"],
  blue: ["#1f3a66", "#172d52"],
  green: ["#1f5a33", "#174628"],
  magenta: ["#5a1f55", "#461743"],
  light: ["#c8ccd4", "#b4b9c2"],
};

function BackdropPicker() {
  const [name, setName] = useState(() => localStorage.getItem("backdrop") ?? "grey");
  useEffect(() => {
    const [a, b] = BACKDROPS[name] ?? BACKDROPS.grey;
    document.documentElement.style.setProperty("--check-a", a);
    document.documentElement.style.setProperty("--check-b", b);
    localStorage.setItem("backdrop", name);
  }, [name]);
  return (
    <span className="backdrop-picker">
      <span className="muted small">background</span>
      {Object.entries(BACKDROPS).map(([n, [a, b]]) => (
        <button
          key={n}
          title={n}
          className={n === name ? "active" : ""}
          style={{ background: `repeating-conic-gradient(${a} 0% 25%, ${b} 0% 50%) 0 0 / 8px 8px` }}
          onClick={() => setName(n)}
        />
      ))}
    </span>
  );
}

export default function App() {
  const m = useHash().match(/^#\/task\/(.+)$/);
  const task = m ? decodeURIComponent(m[1]) : null;
  return (
    <div className="app">
      <header className="topbar">
        <a href="#/">Faery Tale asset review</a>
        {task && <span className="crumb">/ {task}</span>}
        <BackdropPicker />
      </header>
      <main>{task ? <TaskPage key={task} task={task} /> : <TaskList />}</main>
    </div>
  );
}
