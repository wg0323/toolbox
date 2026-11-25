import { reactive } from "vue";
import type { AppEnv } from "./api";

export interface TaskState {
  id: string;
  title: string;
  status: "running" | "done" | "error";
  lines: string[];
  message: string | null;
  startedAt: number;
}

export const store = reactive({
  isDark: localStorage.getItem("toolbox.theme") !== "light",
  env: null as AppEnv | null,
  tasks: [] as TaskState[],
  taskPanelOpen: false,
  revision: 0,
});

export function toggleTheme() {
  store.isDark = !store.isDark;
  localStorage.setItem("toolbox.theme", store.isDark ? "dark" : "light");
}

export function startTask(id: string, title: string) {
  store.tasks.unshift({
    id,
    title,
    status: "running",
    lines: [],
    message: null,
    startedAt: Date.now(),
  });
  store.taskPanelOpen = true;
}

export function appendTaskLine(id: string, line: string) {
  const task = store.tasks.find((item) => item.id === id);
  if (!task) return;
  task.lines.push(line);
  if (task.lines.length > 2000) {
    task.lines.splice(0, task.lines.length - 2000);
  }
}

export function finishTask(id: string, status: "done" | "error", message: string | null) {
  const task = store.tasks.find((item) => item.id === id);
  if (!task) return;
  task.status = status;
  task.message = message;
}
