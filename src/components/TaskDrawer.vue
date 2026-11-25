<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { NButton, NDrawer, NDrawerContent, NEmpty, NTag, useMessage } from "naive-ui";
import { api } from "../api";
import { store } from "../store";

const message = useMessage();
const logEl = ref<HTMLElement | null>(null);

watch(
  () => store.tasks[0]?.lines.length,
  async () => {
    await nextTick();
    if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight;
  },
);

async function cancel(id: string) {
  try {
    await api.taskCancel(id);
  } catch (error) {
    message.error(String(error));
  }
}

function statusLabel(status: string) {
  if (status === "running") return "进行中";
  if (status === "done") return "完成";
  return "失败";
}
</script>

<template>
  <NDrawer v-model:show="store.taskPanelOpen" :width="560" placement="right">
    <NDrawerContent title="任务" closable>
      <NEmpty v-if="store.tasks.length === 0" description="暂无任务" />
      <div v-for="(task, index) in store.tasks" :key="task.id" class="task">
        <div class="task-head">
          <span class="task-title">{{ task.title }}</span>
          <NTag
            size="small"
            :type="task.status === 'done' ? 'success' : task.status === 'error' ? 'error' : 'info'"
          >
            {{ statusLabel(task.status) }}
          </NTag>
          <NButton
            v-if="task.status === 'running'"
            size="tiny"
            quaternary
            type="error"
            @click="cancel(task.id)"
          >
            取消
          </NButton>
        </div>
        <div v-if="task.message" class="task-message">{{ task.message }}</div>
        <pre :ref="index === 0 ? (el) => (logEl = el as HTMLElement) : undefined" class="task-log">{{
          task.lines.join("\n") || "等待输出…"
        }}</pre>
      </div>
    </NDrawerContent>
  </NDrawer>
</template>

<style scoped>
.task {
  margin-bottom: 18px;
}

.task-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.task-title {
  font-size: 13px;
  font-weight: 600;
  flex: 1;
  word-break: break-all;
}

.task-message {
  font-size: 12px;
  opacity: 0.75;
  margin-bottom: 6px;
}

.task-log {
  margin: 0;
  padding: 10px 12px;
  max-height: 320px;
  overflow: auto;
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 11px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  background: rgba(128, 128, 128, 0.1);
  border-radius: 6px;
}
</style>
