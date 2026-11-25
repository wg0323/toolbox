<script setup lang="ts">
import {
  NConfigProvider,
  NDialogProvider,
  NMessageProvider,
  darkTheme,
  dateZhCN,
  lightTheme,
  zhCN,
} from "naive-ui";
import AppShell from "./components/AppShell.vue";
import { store } from "./store";
</script>

<template>
  <NConfigProvider
    :theme="store.isDark ? darkTheme : lightTheme"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <NMessageProvider>
      <NDialogProvider>
        <AppShell />
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>
