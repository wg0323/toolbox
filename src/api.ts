import { invoke } from "@tauri-apps/api/core";

export interface CliOutput {
  success: boolean;
  code: number;
  stdout: string;
  stderr: string;
}

export interface VersionItem {
  version: string;
  active: boolean;
  path: string | null;
}

export interface EnvStatus {
  toolPath: string | null;
  current: string | null;
  linkPath: string | null;
  linkTarget: string | null;
  probe: string | null;
  consistent: boolean;
  warnings: string[];
}

export interface VersionPage {
  status: EnvStatus;
  versions: VersionItem[];
  error: string | null;
}

export interface RemoteVersion {
  version: string;
  group: string | null;
  installed: boolean;
}

export interface PortItem {
  port: number;
  address: string;
  pid: number;
  processName: string;
  exePath: string | null;
  cmdline: string | null;
  startTime: number | null;
  kind: string;
  protected: boolean;
}

export interface KillResult {
  success: boolean;
  message: string;
  released: boolean;
}

export interface AppEnv {
  isAdmin: boolean;
  jvmsPath: string | null;
  nvmPath: string | null;
  javaHome: string | null;
  nvmSymlink: string | null;
}

export const api = {
  appEnv: () => invoke<AppEnv>("app_env"),
  restartAsAdmin: () => invoke<void>("restart_as_admin"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  taskCancel: (taskId: string) => invoke<void>("task_cancel", { taskId }),

  jdkStatus: () => invoke<VersionPage>("jdk_status"),
  jdkSwitch: (version: string) => invoke<CliOutput>("jdk_switch", { version }),
  jdkRemote: (all: boolean) => invoke<RemoteVersion[]>("jdk_remote", { all }),
  jdkInstall: (version: string) => invoke<string>("jdk_install", { version }),
  jdkRemove: (version: string) => invoke<CliOutput>("jdk_remove", { version }),
  jdkProxy: () => invoke<string>("jdk_proxy"),

  nodeStatus: () => invoke<VersionPage>("node_status"),
  nodeSwitch: (version: string) => invoke<CliOutput>("node_switch", { version }),
  nodeRemote: () => invoke<RemoteVersion[]>("node_remote"),
  nodeInstall: (version: string) => invoke<string>("node_install", { version }),
  nodeUninstall: (version: string) => invoke<CliOutput>("node_uninstall", { version }),
  nodeMirror: () => invoke<[string, string]>("node_mirror"),
  nodeMirrorSet: (nodeUrl: string, npmUrl: string) =>
    invoke<void>("node_mirror_set", { nodeUrl, npmUrl }),

  portsList: () => invoke<PortItem[]>("ports_list"),
  portKill: (pid: number) => invoke<KillResult>("port_kill", { pid }),
};
