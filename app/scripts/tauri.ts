import { existsSync } from "node:fs";
import { cp, rm } from "node:fs/promises";

const args = Bun.argv.slice(2);
const bun = process.execPath;

function isWsl() {
  return Boolean(process.env.WSL_DISTRO_NAME || process.env.WSL_INTEROP);
}

async function run(command: string[], options: Bun.SpawnOptions.OptionsObject = {}) {
  const proc = Bun.spawn(command, {
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
    ...options,
  });
  const exitCode = await proc.exited;
  if (exitCode !== 0) {
    throw new Error(`Command '${command[0]}' exited with code ${exitCode}`);
  }
}

async function output(command: string[]) {
  const proc = Bun.spawn(command, { stdout: "pipe", stderr: "inherit" });
  const text = await new Response(proc.stdout).text();
  const exitCode = await proc.exited;
  if (exitCode !== 0) process.exit(exitCode);
  return text.trim();
}

async function fileText(path: string) {
  return Bun.file(path).text().catch(() => "");
}

function shellQuote(value: string) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

function powershell(command: string[]) {
  return ["/bin/bash", "-lc", command.map(shellQuote).join(" ")];
}

const projectName = "panels-erc-edition";

async function installSwitchInputActivityBridge() {
  const root = "src-tauri/gen/android/app/src/main/java";
  if (!existsSync(root)) return false;
  const files = new Bun.Glob("**/MainActivity.kt").scan({ cwd: root, absolute: true });
  let found = false;
  for await (const path of files) {
    let source = await Bun.file(path).text();
    if (source.includes("SwitchInputBridge.onKeyEvent(event)")) continue;
    if (!source.includes("class MainActivity : TauriActivity()")) continue;
    source = source.replace(
      /^package ([^\r\n]+)\r?\n/m,
      "$&\nimport android.view.KeyEvent\nimport android.view.MotionEvent\nimport dev.pulsarupb.switchinput.plugin.SwitchInputBridge\n",
    );
    source = source.replace(
      /class MainActivity : TauriActivity\(\)\s*\{/,
      `class MainActivity : TauriActivity() {
  override fun dispatchKeyEvent(event: KeyEvent): Boolean =
    SwitchInputBridge.onKeyEvent(event) || super.dispatchKeyEvent(event)

  override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean =
    SwitchInputBridge.onGenericMotionEvent(event) || super.dispatchGenericMotionEvent(event)

  override fun dispatchTouchEvent(event: MotionEvent): Boolean =
    SwitchInputBridge.onTouchEvent(event) || super.dispatchTouchEvent(event)
`,
    );
    await Bun.write(path, source);
    found = true;
  }
  return found;
}

async function includeSwitchInputAndroidPlugin() {
  const settingsPath = "src-tauri/gen/android/tauri.settings.gradle";
  const appGradlePath = "src-tauri/gen/android/app/tauri.build.gradle.kts";
  if (!existsSync(settingsPath) || !existsSync(appGradlePath)) return;

  let settings = await Bun.file(settingsPath).text();
  if (!settings.includes("include ':tauri-plugin-switch-input'")) {
    settings += "\ninclude ':tauri-plugin-switch-input'\n";
    settings += "project(':tauri-plugin-switch-input').projectDir = new File(rootProject.projectDir, '../../native/switch-input/android')\n";
    await Bun.write(settingsPath, settings);
  }

  let appGradle = await Bun.file(appGradlePath).text();
  if (!appGradle.includes('implementation(project(":tauri-plugin-switch-input"))')) {
    appGradle = appGradle.replace(
      /\ndependencies\s*\{/,
      '\ndependencies {\n  implementation(project(":tauri-plugin-switch-input"))',
    );
    await Bun.write(appGradlePath, appGradle);
  }
}

async function enforceAndroidLandscapeOrientation() {
  const manifestPath = "src-tauri/gen/android/app/src/main/AndroidManifest.xml";
  if (!existsSync(manifestPath)) return;

  const manifest = await Bun.file(manifestPath).text();
  const activityTag = manifest.match(
    /<activity\b(?=[^>]*\bandroid:name="[^"]*\.MainActivity")[^>]*>/,
  )?.[0];
  if (!activityTag) {
    throw new Error(`Could not find the MainActivity declaration in ${manifestPath}`);
  }

  const landscapeTag = activityTag.includes("android:screenOrientation=")
    ? activityTag.replace(/android:screenOrientation="[^"]*"/, 'android:screenOrientation="sensorLandscape"')
    : activityTag.replace(/>$/, ' android:screenOrientation="sensorLandscape">');
  if (landscapeTag !== activityTag) {
    await Bun.write(manifestPath, manifest.replace(activityTag, landscapeTag));
  }
}

async function installAndroidAppIcons() {
  const source = "src-tauri/icons/android";
  const resources = "src-tauri/gen/android/app/src/main/res";
  if (!existsSync(source) || !existsSync(resources)) return;

  for (const directory of [
    "mipmap-anydpi-v26",
    "mipmap-hdpi",
    "mipmap-mdpi",
    "mipmap-xhdpi",
    "mipmap-xxhdpi",
    "mipmap-xxxhdpi",
  ]) {
    await cp(`${source}/${directory}`, `${resources}/${directory}`, {
      recursive: true,
      force: true,
    });
  }
}

async function syncAndroidAppName() {
  const stringsPath = "src-tauri/gen/android/app/src/main/res/values/strings.xml";
  if (!existsSync(stringsPath)) return;

  const name = "Panels ERC Edition";
  const strings = await Bun.file(stringsPath).text();
  const updated = strings
    .replace(/(<string name="app_name">)[\s\S]*?(<\/string>)/, `$1${name}$2`)
    .replace(/(<string name="main_activity_title">)[\s\S]*?(<\/string>)/, `$1${name}$2`);
  await Bun.write(stringsPath, updated);
}

function windowsTargetCleanupCommand() {
  return [
    `$target = Join-Path $env:LOCALAPPDATA '${projectName}\\target-windows'`,
    "Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($target, [StringComparison]::OrdinalIgnoreCase) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }",
  ].join("; ");
}

function startWindowsCleanupMonitor(parentPid: number, windowsPidPath: string) {
  const cleanupTargetProcesses = [
    "powershell.exe", "-NoProfile", "-Command",
    windowsTargetCleanupCommand(),
  ].map(shellQuote).join(" ");
  const script = [
    `parent_pid=${shellQuote(String(parentPid))}`,
    `windows_pid_path=${shellQuote(windowsPidPath)}`,
    'while kill -0 "$parent_pid" 2>/dev/null; do sleep 1; done',
    'IFS= read -r windows_pid < "$windows_pid_path" 2>/dev/null || exit 0',
    'case "$windows_pid" in (*[!0-9]*|"") ;; (*) taskkill.exe /PID "$windows_pid" /T /F >/dev/null 2>&1 || true;; esac',
    `${cleanupTargetProcesses} >/dev/null 2>&1 || true`,
  ].join("; ");
  Bun.spawn(["/bin/bash", "-lc", script], {
    stdin: "ignore", stdout: "ignore", stderr: "ignore", detached: true,
  });
}

function stopWindowsTargetProcesses() {
  Bun.spawnSync(powershell([
    "powershell.exe", "-NoProfile", "-Command",
    windowsTargetCleanupCommand(),
  ]), { stdout: "ignore", stderr: "ignore" });
}

function canLaunchWindowsPowerShell() {
  try {
    return Bun.spawnSync(
      powershell(["powershell.exe", "-NoProfile", "-Command", "Write-Output ok"]),
    ).success;
  } catch { return false; }
}

async function waitForUrl(url: string, timeoutMs: number) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {}
    await Bun.sleep(250);
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function isPortAvailable(port: number) {
  try {
    const server = Bun.listen({ hostname: "127.0.0.1", port, socket: { data() {} } });
    server.stop(true);
    return true;
  } catch { return false; }
}

async function findAvailablePort(start: number) {
  for (let port = start; port < start + 100; port++) {
    if (await isPortAvailable(port)) return port;
  }
  throw new Error(`No available port found between ${start} and ${start + 99}`);
}

async function runWindowsTauriDev() {
  const requestedPort = Number(process.env.TAURI_DEV_PORT || 5173);
  if (!Number.isInteger(requestedPort) || requestedPort < 1 || requestedPort > 65_535) {
    throw new Error("TAURI_DEV_PORT must be an integer between 1 and 65535");
  }

  const port = await findAvailablePort(requestedPort);
  const devUrl = `http://localhost:${port}`;
  let configPath: string | undefined;
  let windowsPidPath: string | undefined;
  let tauri: Bun.Subprocess | undefined;
  let shuttingDown = false;

  if (!canLaunchWindowsPowerShell()) {
    console.error([
      "WSL cannot launch Windows executables from this session.",
      "Enable WSL interop, then restart WSL:",
      "  1. Ensure /etc/wsl.conf contains:",
      "     [interop]",
      "     enabled=true",
      "     appendWindowsPath=true",
      "  2. From Windows PowerShell, run: wsl --shutdown",
      "  3. Open WSL again and retry: bun run tauri dev",
    ].join("\n"));
    process.exit(1);
  }

  // Step A: Start Vite dev server in WSL
  const vite = Bun.spawn([bun, "run", "dev", "--", "--port", String(port), "--strictPort"], {
    stdin: "ignore", stdout: "inherit", stderr: "inherit",
  });

  const stopVite = () => { vite.kill(); };

  const stopTauri = async () => {
    if (windowsPidPath) {
      const windowsPid = Number((await fileText(windowsPidPath)).trim());
      if (Number.isInteger(windowsPid) && windowsPid > 0) {
        Bun.spawnSync(["taskkill.exe", "/PID", String(windowsPid), "/T", "/F"], {
          stdout: "ignore", stderr: "ignore",
        });
      }
    }
    stopWindowsTargetProcesses();
    tauri?.kill();
  };

  const cleanup = async () => {
    await stopTauri();
    stopVite();
    if (configPath) await Bun.file(configPath).delete().catch(() => {});
    if (windowsPidPath) await Bun.file(windowsPidPath).delete().catch(() => {});
  };

  const shutdown = (exitCode: number) => {
    if (shuttingDown) return;
    shuttingDown = true;
    void cleanup().finally(() => process.exit(exitCode));
  };

  process.on("exit", stopVite);
  process.on("SIGINT", () => shutdown(130));
  process.on("SIGTERM", () => shutdown(143));

  try {
    // Step B: Wait for Vite to be ready
    await Promise.race([
      waitForUrl(devUrl, 30_000),
      vite.exited.then((exitCode) => {
        throw new Error(`Vite exited before ${devUrl} was available with code ${exitCode}`);
      }),
    ]);

    // Step C: Convert WSL paths to Windows paths
    const windowsPath = await output(["wslpath", "-w", process.cwd()]);
    const tempPrefix = `/tmp/${projectName}-tauri-wsl-windows-dev-${process.pid}`;
    configPath = `${tempPrefix}.json`;
    windowsPidPath = `${tempPrefix}.pid`;
    startWindowsCleanupMonitor(process.pid, windowsPidPath);

    // Step D: Generate a stripped-down Tauri config for Windows (no beforeDevCommand)
    await Bun.write(
      configPath,
      `${JSON.stringify({ build: { beforeDevCommand: "", devUrl } }, null, 2)}\n`,
    );

    const windowsConfigPath = await output(["wslpath", "-w", configPath]);
    const windowsPidFilePath = await output(["wslpath", "-w", windowsPidPath]);

    // Step E: Verify tauri-cli is installed on Windows
    const hasWindowsTauriCli = Bun.spawnSync(powershell([
      "powershell.exe", "-NoProfile", "-Command",
      "cargo tauri --version *> $null",
    ])).success;

    if (!hasWindowsTauriCli) {
      console.error([
        "Windows cargo-tauri is not installed.",
        "Install it from Windows PowerShell with:",
        "  cargo install tauri-cli --version ^2",
      ].join("\n"));
      process.exit(1);
    }

    // Step F: Launch Tauri on Windows
    const command = [
      "$ErrorActionPreference = 'Stop'",
      `Set-Content -LiteralPath '${windowsPidFilePath.replaceAll("'", "''")}' -Value $PID`,
      `Set-Location -LiteralPath '${windowsPath.replaceAll("'", "''")}'`,
      `$env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA '${projectName}\\target-windows'`,
      `cargo tauri dev --no-dev-server --config '${windowsConfigPath.replaceAll("'", "''")}'`,
    ].join("; ");

    tauri = Bun.spawn(powershell([
      "powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command,
    ]), {
      stdin: "ignore", stdout: "inherit", stderr: "inherit",
    });

    const exitCode = await tauri.exited;
    if (!shuttingDown && exitCode !== 0) process.exitCode = exitCode;
  } finally {
    await cleanup();
  }
}

// Main entry
if (args[0] === "dev" && isWsl()) {
  await runWindowsTauriDev();
} else {
  if (args[0] === "android") {
    const switchEdition = process.env.ANDROID_APP_FLAVOR !== "standard";
    process.env.VITE_GAMEPAD_TRANSPORT ||= switchEdition ? "native" : "web";
    if (switchEdition) {
      await installSwitchInputActivityBridge();
      await includeSwitchInputAndroidPlugin();
    }
    await enforceAndroidLandscapeOrientation();
    if (args.includes("build")) {
      await installAndroidAppIcons();
      await syncAndroidAppName();
    }
    const switchCapability = "src-tauri/capabilities/default-switch.json";
    const needsSwitchCapability = switchEdition && args.includes("build");
    const createdSwitchCapability = needsSwitchCapability && !existsSync(switchCapability);
    if (createdSwitchCapability) {
      await cp("src-tauri/capability-templates/default-switch.json", switchCapability);
    }
    try {
      await run([bun, "x", "tauri", ...args]);
      if (args.includes("init")) {
        if (switchEdition) {
          await installSwitchInputActivityBridge();
          await includeSwitchInputAndroidPlugin();
        }
        await enforceAndroidLandscapeOrientation();
        await installAndroidAppIcons();
        await syncAndroidAppName();
      }
    } finally {
      if (createdSwitchCapability) await rm(switchCapability, { force: true });
    }
  } else {
    await run([bun, "x", "tauri", ...args]);
  }
}
