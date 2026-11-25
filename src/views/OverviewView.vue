<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { NButton, NCard, NSpin, useMessage } from "naive-ui";
import { api, type PortItem, type VersionPage } from "../api";
import { store } from "../store";

const emit = defineEmits<{ navigate: [key: string] }>();
const message = useMessage();

const jdk = ref<VersionPage | null>(null);
const node = ref<VersionPage | null>(null);
const ports = ref<PortItem[]>([]);
const loading = ref(false);

const springPorts = computed(() => ports.value.filter((item) => item.kind === "spring-boot"));
const javaPorts = computed(() =>
  ports.value.filter((item) => ["spring-boot", "java", "java-ide"].includes(item.kind)),
);

async function load() {
  loading.value = true;
  try {
    const [jdkPage, nodePage, portList] = await Promise.all([
      api.jdkStatus(),
      api.nodeStatus(),
      api.portsList(),
    ]);
    jdk.value = jdkPage;
    node.value = nodePage;
    ports.value = portList;
  } catch (error) {
    message.error(String(error));
  } finally {
    loading.value = false;
  }
}

watch(() => store.revision, load);

onMounted(load);
</script>

<template>
  <div>
    <div class="page-head">
      <div>
        <h2 class="page-title">概览</h2>
        <div class="hint">当前机器上实际生效的 JDK、Node 与 Java 服务端口</div>
      </div>
      <NButton size="small" :loading="loading" @click="load">刷新</NButton>
    </div>

    <NSpin :show="loading">
      <div class="cards">
        <NCard size="small" class="card" hoverable @click="emit('navigate', 'jdk')">
          <div class="card-head">
            <span class="card-title">JDK</span>
            <span class="status-tag">当前生效</span>
          </div>
          <div class="big mono">{{ jdk?.status.current ?? "未识别" }}</div>
        </NCard>

        <NCard size="small" class="card" hoverable @click="emit('navigate', 'node')">
          <div class="card-head">
            <span class="card-title">Node</span>
            <span class="status-tag">当前生效</span>
          </div>
          <div class="big mono">{{ node?.status.current ?? "未识别" }}</div>
        </NCard>

        <NCard size="small" class="card" hoverable @click="emit('navigate', 'ports')">
          <div class="card-head">
            <span class="card-title">端口</span>
          </div>
          <div class="big">{{ javaPorts.length }} <span class="unit">个 Java 监听</span></div>
          <div class="card-line">全部 TCP 监听 {{ ports.length }} 个</div>
          <div class="card-line mono">
            {{ springPorts.slice(0, 6).map((item) => item.port).join("  ") || "暂无 Spring Boot 端口" }}
          </div>
        </NCard>
      </div>

      <NCard size="small" class="guide">
        <div class="guide-title">常用动作</div>
        <div class="guide-actions">
          <NButton size="small" @click="emit('navigate', 'ports')">查 8080 被谁占用</NButton>
          <NButton size="small" @click="emit('navigate', 'jdk')">切换 JDK 版本</NButton>
          <NButton size="small" @click="emit('navigate', 'node')">切换 Node 版本</NButton>
        </div>
      </NCard>
    </NSpin>
  </div>
</template>

<style scoped>
.cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
  margin-bottom: 14px;
}

.card {
  cursor: pointer;
}

.card-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.3px;
}

.big {
  font-size: 26px;
  font-weight: 600;
  margin-bottom: 8px;
}

.unit {
  font-size: 12px;
  font-weight: 400;
  opacity: 0.6;
}

.card-line {
  font-size: 11px;
  opacity: 0.65;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

.guide-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 10px;
}

.guide-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
</style>
