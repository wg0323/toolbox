<script setup lang="ts">
import { computed, h, onMounted, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCard,
  NCollapse,
  NCollapseItem,
  NDataTable,
  NInput,
  NModal,
  NPopconfirm,
  NSpin,
  NSwitch,
  NTag,
  useMessage,
} from "naive-ui";
import type { DataTableColumns } from "naive-ui";
import { api, type RemoteVersion, type VersionItem, type VersionPage } from "../api";
import { startTask, store } from "../store";

const props = defineProps<{ kind: "jdk" | "node"; title: string }>();
const message = useMessage();

const page = ref<VersionPage | null>(null);
const loading = ref(false);
const pendingSwitch = ref<VersionItem | null>(null);
const switching = ref(false);

const remoteShow = ref(false);
const remoteLoading = ref(false);
const remoteError = ref("");
const remoteAll = ref(false);
const remoteKeyword = ref("");
const remoteOnlyNew = ref(false);
const remoteList = ref<RemoteVersion[]>([]);

const mirror = ref<[string, string]>(["", ""]);
const mirrorLoading = ref(false);
const proxyInfo = ref("");

const isJdk = computed(() => props.kind === "jdk");

const loadPage = isJdk.value ? api.jdkStatus : api.nodeStatus;
const doSwitch = isJdk.value ? api.jdkSwitch : api.nodeSwitch;
const doRemove = isJdk.value ? api.jdkRemove : api.nodeUninstall;

async function load() {
  loading.value = true;
  try {
    page.value = await loadPage();
  } catch (error) {
    message.error(String(error));
  } finally {
    loading.value = false;
  }
}

async function loadRemote() {
  remoteLoading.value = true;
  remoteError.value = "";
  try {
    remoteList.value = isJdk.value ? await api.jdkRemote(remoteAll.value) : await api.nodeRemote();
  } catch (error) {
    remoteError.value = String(error);
    remoteList.value = [];
  } finally {
    remoteLoading.value = false;
  }
}

async function openRemote() {
  remoteShow.value = true;
  await loadRemote();
}

const remoteFiltered = computed(() => {
  const key = remoteKeyword.value.trim().toLowerCase();
  return remoteList.value.filter((item) => {
    if (remoteOnlyNew.value && item.installed) return false;
    if (!key) return true;
    return item.version.toLowerCase().includes(key);
  });
});

async function install(version: string) {
  try {
    const taskId = isJdk.value
      ? await api.jdkInstall(version)
      : await api.nodeInstall(version);
    startTask(taskId, `安装 ${isJdk.value ? "JDK" : "Node"} ${version}`);
    remoteShow.value = false;
  } catch (error) {
    message.error(String(error));
  }
}

async function confirmSwitch() {
  if (!pendingSwitch.value) return;
  const version = pendingSwitch.value.version;
  switching.value = true;
  try {
    await doSwitch(version);
    message.success(`已切换到 ${version}，新启动的进程生效`);
    pendingSwitch.value = null;
    await load();
  } catch (error) {
    message.error(String(error));
  } finally {
    switching.value = false;
  }
}

async function remove(version: string) {
  try {
    await doRemove(version);
    message.success(`已卸载 ${version}`);
    await load();
  } catch (error) {
    message.error(String(error));
  }
}

async function loadMirror() {
  mirrorLoading.value = true;
  try {
    mirror.value = await api.nodeMirror();
  } catch (error) {
    message.error(String(error));
  } finally {
    mirrorLoading.value = false;
  }
}

async function applyMirror(nodeUrl: string, npmUrl: string) {
  try {
    await api.nodeMirrorSet(nodeUrl, npmUrl);
    message.success("镜像已设置");
    await loadMirror();
  } catch (error) {
    message.error(String(error));
  }
}

async function loadProxy() {
  try {
    proxyInfo.value = await api.jdkProxy();
  } catch (error) {
    proxyInfo.value = String(error);
  }
}

const columns = computed<DataTableColumns<VersionItem>>(() => [
  {
    title: "版本",
    key: "version",
    width: 140,
    render: (row) =>
      h("span", { class: "mono" }, row.version),
  },
  {
    title: "状态",
    key: "active",
    width: 96,
    render: (row) =>
      row.active
        ? h(NTag, { size: "small", type: "success" }, { default: () => "使用中" })
        : h("span", { class: "hint" }, "—"),
  },
  {
    title: "安装位置",
    key: "path",
    ellipsis: { tooltip: true },
    render: (row) => h("span", { class: "mono hint" }, row.path ?? "—"),
  },
  {
    title: "操作",
    key: "actions",
    width: 150,
    render: (row) =>
      h("div", { class: "row-actions" }, [
        h(
          NButton,
          {
            size: "tiny",
            quaternary: true,
            disabled: row.active,
            onClick: () => (pendingSwitch.value = row),
          },
          { default: () => "切换" },
        ),
        h(
          NPopconfirm,
          { onPositiveClick: () => remove(row.version) },
          {
            trigger: () =>
              h(
                NButton,
                { size: "tiny", quaternary: true, type: "error", disabled: row.active },
                { default: () => "卸载" },
              ),
            default: () => `确认删除 ${row.version} 的安装文件？`,
          },
        ),
      ]),
  },
]);

watch(() => store.revision, load);

onMounted(() => {
  load();
  if (props.kind === "node") loadMirror();
});
</script>

<template>
  <div>
    <div class="page-head">
      <div>
        <h2 class="page-title">{{ title }}</h2>
        <div class="hint">切换后新启动的进程生效，已运行的进程不受影响</div>
      </div>
      <div class="page-actions">
        <NButton size="small" type="primary" @click="openRemote">安装版本</NButton>
        <NButton size="small" :loading="loading" @click="load">刷新</NButton>
      </div>
    </div>

    <NSpin :show="loading">
      <NCard v-if="page" size="small" class="status-card">
        <div class="status-top">
          <span class="current mono">{{ page.status.current ?? "未识别" }}</span>
          <span class="status-tag">当前生效</span>
        </div>

        <NAlert
          v-for="(warning, index) in page.status.warnings"
          :key="index"
          type="warning"
          :bordered="false"
          size="small"
          class="warning"
        >
          {{ warning }}
        </NAlert>
        <NAlert v-if="page.error" type="error" :bordered="false" size="small" class="warning">
          {{ page.error }}
        </NAlert>
      </NCard>

      <NDataTable
        v-if="page"
        class="version-table"
        :columns="columns"
        :data="page.versions"
        :row-key="(row: VersionItem) => row.version"
        size="small"
        :bordered="false"
      />

      <NCollapse v-if="kind === 'node'" class="extra">
        <NCollapseItem title="下载镜像（影响 nvm install 速度）" name="mirror">
          <div v-if="mirrorLoading" class="hint">读取中…</div>
          <template v-else>
            <div class="mirror-line mono">node: {{ mirror[0] || "默认（nodejs.org）" }}</div>
            <div class="mirror-line mono">npm: {{ mirror[1] || "默认（github.com）" }}</div>
            <div class="mirror-actions">
              <NButton
                size="tiny"
                @click="applyMirror('https://npmmirror.com/mirrors/node/', 'https://npmmirror.com/mirrors/npm/')"
              >
                切换为 npmmirror
              </NButton>
            </div>
            <p class="hint">设置会写入 nvm 的 settings.txt，可随时改回默认。</p>
          </template>
        </NCollapseItem>
      </NCollapse>

      <NCollapse v-if="kind === 'jdk'" class="extra">
        <NCollapseItem title="远程索引与代理" name="proxy">
          <div class="mirror-line mono">{{ proxyInfo || "点击下方按钮读取当前代理" }}</div>
          <div class="mirror-actions">
            <NButton size="tiny" @click="loadProxy">读取代理</NButton>
          </div>
          <p class="hint">
            jvms 从 raw.githubusercontent.com 拉取版本索引，国内网络常不可达；安装失败时先在终端为 jvms 配置代理。
          </p>
        </NCollapseItem>
      </NCollapse>
    </NSpin>

    <NModal
      :show="remoteShow"
      preset="card"
      title="安装版本"
      style="width: 720px"
      @update:show="(value: boolean) => (remoteShow = value)"
    >
      <div class="remote-toolbar">
        <NInput v-model:value="remoteKeyword" clearable placeholder="搜索版本号" style="width: 200px" />
        <template v-if="isJdk">
          <span class="hint">列出全部</span>
          <NSwitch v-model:value="remoteAll" @update:value="loadRemote" />
        </template>
        <span class="hint">只看未安装</span>
        <NSwitch v-model:value="remoteOnlyNew" />
        <div class="spacer" />
        <NButton size="small" :loading="remoteLoading" @click="loadRemote">重新加载</NButton>
      </div>

      <NAlert v-if="remoteError" type="error" :bordered="false" class="warning">
        {{ remoteError }}
      </NAlert>

      <div class="remote-list">
        <NSpin :show="remoteLoading">
          <div v-for="item in remoteFiltered" :key="item.version" class="remote-row">
            <span class="mono remote-version">{{ item.version }}</span>
            <NTag v-if="item.group" size="small">{{ item.group }}</NTag>
            <NTag v-if="item.installed" size="small" type="success">已安装</NTag>
            <div class="spacer" />
            <NButton
              size="tiny"
              type="primary"
              secondary
              :disabled="item.installed"
              @click="install(item.version)"
            >
              安装
            </NButton>
          </div>
          <div v-if="!remoteLoading && remoteFiltered.length === 0" class="hint remote-empty">
            没有可显示的版本
          </div>
        </NSpin>
      </div>
    </NModal>

    <NModal
      :show="Boolean(pendingSwitch)"
      preset="dialog"
      type="warning"
      title="确认切换版本"
      positive-text="切换"
      negative-text="取消"
      :loading="switching"
      @positive-click="confirmSwitch"
      @negative-click="pendingSwitch = null"
      @update:show="(value: boolean) => { if (!value) pendingSwitch = null; }"
    >
      <p class="hint">
        {{ isJdk ? "需要管理员权限。" : "nvm 会修改符号链接，可能弹出 UAC 确认。" }}
      </p>
    </NModal>
  </div>
</template>

<style scoped>
.status-card {
  margin-bottom: 14px;
}

.status-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.current {
  font-size: 24px;
  font-weight: 600;
}

.warning {
  margin-top: 8px;
}

.version-table {
  margin-bottom: 14px;
}

.row-actions {
  display: flex;
  gap: 2px;
}

.extra {
  margin-top: 8px;
}

.mirror-line {
  font-size: 12px;
  margin-bottom: 6px;
  word-break: break-all;
}

.mirror-actions {
  margin: 8px 0;
}

.remote-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.spacer {
  flex: 1;
}

.remote-list {
  max-height: 420px;
  overflow: auto;
  margin-top: 8px;
}

.remote-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 4px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.12);
}

.remote-version {
  width: 160px;
}

.remote-empty {
  padding: 24px 0;
  text-align: center;
}
</style>
