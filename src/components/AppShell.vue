<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { NButton, NLayout, NLayoutContent, NLayoutSider, NMenu, NTag, useMessage } from "naive-ui";
import type { MenuOption } from "naive-ui";
import { listen } from "@tauri-apps/api/event";
import { api } from "../api";
import { appendTaskLine, finishTask, store } from "../store";
import TaskDrawer from "./TaskDrawer.vue";
import OverviewView from "../views/OverviewView.vue";
import JdkView from "../views/JdkView.vue";
import NodeView from "../views/NodeView.vue";
import PortsView from "../views/PortsView.vue";
import SettingsView from "../views/SettingsView.vue";

interface TaskEvent {
  taskId: string;
  kind: "line" | "done" | "error";
  stream: string;
  line: string | null;
  code: number | null;
  message: string | null;
}

const message = useMessage();
const current = ref("overview");

const menuOptions: MenuOption[] = [
  { label: "概览", key: "overview" },
  { label: "JDK 管理", key: "jdk" },
  { label: "Node 管理", key: "node" },
  { label: "端口占用", key: "ports" },
  { label: "设置", key: "settings" },
];

const views: Record<string, unknown> = {
  overview: OverviewView,
  jdk: JdkView,
  node: NodeView,
  ports: PortsView,
  settings: SettingsView,
};

const activeView = computed(() => views[current.value]);

let unlisten: (() => void) | null = null;

async function loadEnv() {
  try {
    store.env = await api.appEnv();
  } catch (error) {
    message.error(String(error));
  }
}

async function restartAsAdmin() {
  try {
    await api.restartAsAdmin();
  } catch (error) {
    message.error(String(error));
  }
}

onMounted(async () => {
  await loadEnv();
  unlisten = await listen<TaskEvent>("task://event", (event) => {
    const payload = event.payload;
    if (payload.kind === "line") {
      const prefix = payload.stream === "stderr" ? "! " : "";
      appendTaskLine(payload.taskId, `${prefix}${payload.line ?? ""}`);
      return;
    }
    const ok = payload.kind === "done";
    finishTask(payload.taskId, ok ? "done" : "error", payload.message);
    if (ok) {
      message.success("任务完成");
    } else {
      message.error(payload.message ?? "任务失败");
    }
    store.revision += 1;
  });
});

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <NLayout class="shell" has-sider>
    <NLayoutSider width="220" bordered>
      <div class="brand">
        <span class="brand-name">Toolbox</span>
      </div>
      <NMenu v-model:value="current" :options="menuOptions" />
      <div class="sider-foot">
        <div class="sider-row">
          <span class="muted">权限</span>
          <NTag size="small" :type="store.env?.isAdmin ? 'success' : 'warning'">
            {{ store.env?.isAdmin ? "管理员" : "普通用户" }}
          </NTag>
        </div>
        <div class="sider-path mono" :title="store.env?.jvmsPath ?? ''">
          jvms: {{ store.env?.jvmsPath ?? "未找到" }}
        </div>
        <div class="sider-path mono" :title="store.env?.nvmPath ?? ''">
          nvm: {{ store.env?.nvmPath ?? "未找到" }}
        </div>
      </div>
    </NLayoutSider>

    <NLayoutContent class="content">
      <div v-if="store.env && !store.env.isAdmin" class="admin-bar">
        <span>普通用户运行：JDK 切换与安装需要管理员权限，Node 切换会弹出 UAC</span>
        <NButton size="tiny" type="primary" @click="restartAsAdmin">以管理员身份重启</NButton>
      </div>
      <div class="page">
        <component :is="activeView" @navigate="current = $event" />
      </div>
    </NLayoutContent>

    <TaskDrawer />
  </NLayout>
</template>

<style scoped>
.shell {
  height: 100vh;
}

.brand {
  padding: 18px 20px 12px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.brand-name {
  font-size: 17px;
  font-weight: 600;
  letter-spacing: 0.4px;
}

.sider-foot {
  position: absolute;
  bottom: 0;
  width: 100%;
  padding: 12px 16px 16px;
  border-top: 1px solid rgba(128, 128, 128, 0.18);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.sider-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.muted {
  font-size: 12px;
  opacity: 0.6;
}

.sider-path {
  font-size: 11px;
  opacity: 0.5;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.content {
  height: 100vh;
  overflow: auto;
}

.admin-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 24px;
  font-size: 12px;
  background: rgba(240, 160, 32, 0.14);
  border-bottom: 1px solid rgba(240, 160, 32, 0.3);
}

.page {
  padding: 20px 24px 32px;
}
</style>
