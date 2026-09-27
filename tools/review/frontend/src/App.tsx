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

export default function App() {
  const m = useHash().match(/^#\/task\/(.+)$/);
  const task = m ? decodeURIComponent(m[1]) : null;
  return (
    <div className="app">
      <header className="topbar">
        <a href="#/">Faery Tale asset review</a>
        {task && <span className="crumb">/ {task}</span>}
      </header>
      <main>{task ? <TaskPage key={task} task={task} /> : <TaskList />}</main>
    </div>
  );
}
