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
  if (exitCode !== 0) process.exit(exitCode);
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
  await run([bun, "x", "tauri", ...args]);
}
