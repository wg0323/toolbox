<script setup lang="ts">
import { NButton, NCard, NDescriptions, NDescriptionsItem, NSwitch, NTag, useMessage } from "naive-ui";
import { api } from "../api";
import { store, toggleTheme } from "../store";

const message = useMessage();

async function restartAsAdmin() {
  try {
    await api.restartAsAdmin();
  } catch (error) {
    message.error(String(error));
  }
}

async function openPath(path: string | null) {
  if (!path) return;
  try {
    await api.openPath(path);
  } catch (error) {
    message.error(String(error));
  }
}
</script>

<template>
  <div>
    <div class="page-head">
      <div>
        <h2 class="page-title">设置</h2>
        <div class="hint">工具路径由 PATH 自动探测，这里只做展示与诊断</div>
      </div>
    </div>

    <NCard size="small" class="block">
      <div class="block-title">运行环境</div>
      <NDescriptions :column="1" label-placement="left" label-style="width: 120px">
        <NDescriptionsItem label="权限">
          <NTag size="small" :type="store.env?.isAdmin ? 'success' : 'warning'">
            {{ store.env?.isAdmin ? "管理员" : "普通用户" }}
          </NTag>
          <NButton
            v-if="store.env && !store.env.isAdmin"
            size="tiny"
            class="inline-btn"
            @click="restartAsAdmin"
          >
            以管理员身份重启
          </NButton>
        </NDescriptionsItem>
        <NDescriptionsItem label="jvms">
          <span class="mono">{{ store.env?.jvmsPath ?? "未找到" }}</span>
          <NButton size="tiny" quaternary :disabled="!store.env?.jvmsPath" @click="openPath(store.env?.jvmsPath ?? null)">
            打开目录
          </NButton>
        </NDescriptionsItem>
        <NDescriptionsItem label="nvm">
          <span class="mono">{{ store.env?.nvmPath ?? "未找到" }}</span>
          <NButton size="tiny" quaternary :disabled="!store.env?.nvmPath" @click="openPath(store.env?.nvmPath ?? null)">
            打开目录
          </NButton>
        </NDescriptionsItem>
        <NDescriptionsItem label="JAVA_HOME">
          <span class="mono">{{ store.env?.javaHome ?? "未设置" }}</span>
        </NDescriptionsItem>
        <NDescriptionsItem label="NVM_SYMLINK">
          <span class="mono">{{ store.env?.nvmSymlink ?? "未设置" }}</span>
        </NDescriptionsItem>
      </NDescriptions>

      <p class="hint note">
        环境变量在应用启动时读取。若你刚刚修改过系统环境变量，需要重启本应用才会刷新。
      </p>
    </NCard>

    <NCard size="small" class="block">
      <div class="block-title">外观</div>
      <div class="row">
        <span>深色主题</span>
        <NSwitch :value="store.isDark" @update:value="toggleTheme" />
      </div>
    </NCard>

    <NCard size="small" class="block">
      <div class="block-title">说明</div>
      <ul class="notes">
        <li>JDK 切换与安装调用 jvms，会改写系统 JAVA_HOME 与 C:\Program Files\jdk 符号链接，必须管理员权限。</li>
        <li>Node 切换与安装调用 nvm-windows，nvm 自带提权脚本，普通权限下会弹出 UAC。</li>
        <li>端口页只枚举 TCP 监听，结束时强制终止进程，不执行 Spring Boot 的 shutdown hook。</li>
        <li>本应用不修改任何系统环境变量，所有变更都由 jvms / nvm 自己完成。</li>
      </ul>
    </NCard>
  </div>
</template>

<style scoped>
.block {
  margin-bottom: 14px;
}

.block-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 12px;
}

.inline-btn {
  margin-left: 10px;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.note {
  margin-top: 12px;
  margin-bottom: 0;
}

.notes {
  margin: 0;
  padding-left: 18px;
  font-size: 12px;
  line-height: 1.9;
  opacity: 0.8;
}
</style>
