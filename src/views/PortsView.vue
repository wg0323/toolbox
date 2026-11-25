<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref, watch } from "vue";
import {
  NButton,
  NDataTable,
  NDrawer,
  NDrawerContent,
  NEmpty,
  NInput,
  NModal,
  NSelect,
  NSwitch,
  NTag,
  useMessage,
} from "naive-ui";
import type { DataTableColumns } from "naive-ui";
import { api, type PortItem } from "../api";
import { store } from "../store";

const emit = defineEmits<{ navigate: [key: string] }>();
const message = useMessage();

const ports = ref<PortItem[]>([]);
const loading = ref(false);
const keyword = ref("");
const onlyJava = ref(true);
const autoRefresh = ref(5);
const detail = ref<PortItem | null>(null);
const pending = ref<PortItem | null>(null);
const killing = ref(false);

const quickPorts = [8080, 3000, 5173, 3306, 6379];
const refreshOptions = [
  { label: "关闭", value: 0 },
  { label: "3 秒", value: 3 },
  { label: "5 秒", value: 5 },
  { label: "10 秒", value: 10 },
];

const filtered = computed(() => {
  const key = keyword.value.trim().toLowerCase();
  if (key) {
    return ports.value.filter(
      (item) =>
        String(item.port).includes(key) ||
        item.processName.toLowerCase().includes(key) ||
        (item.cmdline ?? "").toLowerCase().includes(key),
    );
  }
  if (!onlyJava.value) return ports.value;
  return ports.value.filter((item) => ["spring-boot", "java", "java-ide"].includes(item.kind));
});

const springCount = computed(
  () => ports.value.filter((item) => item.kind === "spring-boot").length,
);

function kindLabel(kind: string) {
  const labels: Record<string, string> = {
    "spring-boot": "Spring Boot",
    java: "Java",
    "java-ide": "Java (IDE)",
    node: "Node",
    docker: "Docker",
    other: "其他",
  };
  return labels[kind] ?? kind;
}

function kindType(kind: string) {
  if (kind === "spring-boot") return "success" as const;
  if (kind === "java") return "info" as const;
  if (kind === "java-ide") return "warning" as const;
  if (kind === "docker") return "error" as const;
  return "default" as const;
}

function formatDuration(start: number | null) {
  if (!start) return "-";
  const seconds = Math.max(0, Math.floor(Date.now() / 1000 - start));
  if (seconds < 60) return `${seconds} 秒`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} 分`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时 ${minutes % 60} 分`;
  return `${Math.floor(hours / 24)} 天 ${hours % 24} 小时`;
}

const columns = computed<DataTableColumns<PortItem>>(() => [
  {
    title: "端口",
    key: "port",
    width: 80,
    sorter: (a, b) => a.port - b.port,
  },
  {
    title: "类型",
    key: "kind",
    width: 118,
    render: (row) =>
      h(NTag, { size: "small", type: kindType(row.kind) }, { default: () => kindLabel(row.kind) }),
  },
  {
    title: "进程",
    key: "processName",
    minWidth: 140,
    ellipsis: { tooltip: true },
  },
  { title: "PID", key: "pid", width: 82 },
  { title: "监听地址", key: "address", width: 160 },
  {
    title: "运行时长",
    key: "startTime",
    width: 120,
    render: (row) => formatDuration(row.startTime),
  },
  {
    title: "操作",
    key: "actions",
    width: 130,
    render: (row) =>
      h("div", { class: "row-actions" }, [
        h(
          NButton,
          { size: "tiny", quaternary: true, onClick: () => (detail.value = row) },
          { default: () => "详情" },
        ),
        h(
          NButton,
          {
            size: "tiny",
            quaternary: true,
            type: "error",
            disabled: row.protected,
            onClick: () => (pending.value = row),
          },
          { default: () => "结束" },
        ),
      ]),
  },
]);

async function load() {
  loading.value = true;
  try {
    ports.value = await api.portsList();
  } catch (error) {
    message.error(String(error));
  } finally {
    loading.value = false;
  }
}

let timer: number | undefined;
function resetTimer() {
  if (timer) window.clearInterval(timer);
  timer = undefined;
  if (autoRefresh.value > 0) {
    timer = window.setInterval(() => {
      if (!loading.value) load();
    }, autoRefresh.value * 1000);
  }
}

async function confirmKill() {
  if (!pending.value) return;
  const target = pending.value;
  pending.value = null;
  killing.value = true;
  try {
    const result = await api.portKill(target.pid);
    if (result.success) {
      message.success(result.message);
    } else {
      message.error(result.message);
    }
    await load();
  } catch (error) {
    message.error(String(error));
  } finally {
    killing.value = false;
  }
}

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    message.success("已复制");
  } catch {
    message.error("复制失败");
  }
}

watch(autoRefresh, resetTimer);
watch(() => store.revision, load);

onMounted(() => {
  load();
  resetTimer();
});

onUnmounted(() => {
  if (timer) window.clearInterval(timer);
});
</script>

<template>
  <div>
    <div class="page-head">
      <div>
        <h2 class="page-title">端口占用</h2>
        <div class="hint">
          只枚举 TCP 监听端口；默认只看 Java / Spring Boot，按端口搜索时会显示真实占用者
        </div>
      </div>
      <div class="page-actions">
        <NTag size="small" type="success">Spring Boot {{ springCount }}</NTag>
        <NTag size="small">监听总数 {{ ports.length }}</NTag>
        <NButton size="small" :loading="loading" @click="load">刷新</NButton>
      </div>
    </div>

    <div class="toolbar">
      <NInput
        v-model:value="keyword"
        clearable
        placeholder="端口号 / 进程名 / 命令行片段"
        style="width: 280px"
      />
      <div class="quick">
        <NButton
          v-for="port in quickPorts"
          :key="port"
          size="tiny"
          quaternary
          @click="keyword = String(port)"
        >
          {{ port }}
        </NButton>
      </div>
      <div class="spacer" />
      <span class="hint">只看 Java / Spring Boot</span>
      <NSwitch v-model:value="onlyJava" :disabled="Boolean(keyword.trim())" />
      <span class="hint">自动刷新</span>
      <NSelect v-model:value="autoRefresh" :options="refreshOptions" size="small" style="width: 92px" />
    </div>

    <NDataTable
      :columns="columns"
      :data="filtered"
      :loading="loading"
      :row-key="(row: PortItem) => `${row.port}-${row.pid}`"
      size="small"
      :bordered="false"
      :max-height="520"
      virtual-scroll
    >
      <template #empty>
        <NEmpty description="没有匹配的监听端口">
          <template #extra>
            <div class="empty-actions">
              <NButton v-if="onlyJava && !keyword" size="small" @click="onlyJava = false">
                显示全部进程
              </NButton>
              <NButton size="small" @click="load">重新扫描</NButton>
            </div>
          </template>
        </NEmpty>
      </template>
    </NDataTable>

    <NDrawer
      :show="Boolean(detail)"
      :width="520"
      placement="right"
      @update:show="(value: boolean) => { if (!value) detail = null; }"
    >
      <NDrawerContent v-if="detail" title="进程详情" closable>
        <div class="detail-row">
          <span class="detail-label">端口</span>
          <span class="mono">{{ detail.port }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">类型</span>
          <NTag size="small" :type="kindType(detail.kind)">{{ kindLabel(detail.kind) }}</NTag>
        </div>
        <div class="detail-row">
          <span class="detail-label">进程</span>
          <span>{{ detail.processName }}（PID {{ detail.pid }}）</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">监听地址</span>
          <span class="mono">{{ detail.address }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">运行时长</span>
          <span>{{ formatDuration(detail.startTime) }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">可执行文件</span>
          <span class="mono detail-value">{{ detail.exePath ?? "无权限读取" }}</span>
        </div>
        <div class="detail-block">
          <div class="detail-label">命令行</div>
          <pre class="cmd">{{ detail.cmdline ?? "无权限读取（以管理员身份运行后可查看）" }}</pre>
        </div>
        <div class="drawer-actions">
          <NButton size="small" @click="copy(String(detail.pid))">复制 PID</NButton>
          <NButton
            size="small"
            @click="copy(detail.cmdline ?? '')"
            :disabled="!detail.cmdline"
          >
            复制命令行
          </NButton>
          <NButton size="small" @click="api.openPath(detail.exePath ?? '')" :disabled="!detail.exePath">
            打开所在目录
          </NButton>
          <NButton
            size="small"
            type="error"
            :disabled="detail.protected"
            @click="pending = detail; detail = null"
          >
            结束进程
          </NButton>
        </div>
      </NDrawerContent>
    </NDrawer>

    <NModal
      :show="Boolean(pending)"
      preset="dialog"
      type="error"
      title="确认结束进程"
      positive-text="结束"
      negative-text="取消"
      :loading="killing"
      @positive-click="confirmKill"
      @negative-click="pending = null"
      @update:show="(value: boolean) => { if (!value) pending = null; }"
    >
      <div v-if="pending" class="confirm">
        <p>
          <b>{{ pending.processName }}</b>（PID {{ pending.pid }}）正监听端口
          <b>{{ pending.port }}</b>，结束后该端口立即释放。
        </p>
        <p class="hint">
          Java 进程被强制结束时不会执行 Spring Boot 的 shutdown hook，未刷盘的事务可能丢失。
        </p>
        <pre class="cmd">{{ pending.cmdline ?? "命令行不可读" }}</pre>
        <p v-if="pending.kind === 'docker'" class="warn">
          这是 Docker 的端口代理进程，结束它只会中断端口映射，不会停止容器。
        </p>
      </div>
    </NModal>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.quick {
  display: flex;
  gap: 2px;
}

.spacer {
  flex: 1;
}

.row-actions {
  display: flex;
  gap: 2px;
}

.detail-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 5px 0;
  font-size: 13px;
}

.detail-label {
  width: 84px;
  flex-shrink: 0;
  opacity: 0.6;
  font-size: 12px;
}

.detail-value {
  word-break: break-all;
}

.detail-block {
  margin-top: 10px;
}

.cmd {
  margin: 6px 0 0;
  padding: 10px 12px;
  max-height: 220px;
  overflow: auto;
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 11px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  background: rgba(128, 128, 128, 0.1);
  border-radius: 6px;
}

.drawer-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 20px;
}

.confirm p {
  margin: 0 0 8px;
  line-height: 1.7;
}

.warn {
  color: #e0a030;
}

/* 概览页与任务面板也使用该宽度 */
</style>
