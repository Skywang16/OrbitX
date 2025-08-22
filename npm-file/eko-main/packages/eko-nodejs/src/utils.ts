import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { Log } from "@eko-ai/eko";

export async function getCdpWsEndpoint(port: number): Promise<string> {
  // Example => ws://localhost:9222/devtools/browser/{session-id}
  const response = await fetch(`http://localhost:${port}/json/version`);
  const browserInfo = await response.json();
  Log.info("browserInfo: ", browserInfo);
  return browserInfo.webSocketDebuggerUrl as string;
}

export function getDefaultChromeUserDataDir(
  copyToTempDir: boolean = false
): string | undefined {
  const platform = os.platform();
  const homeDir = os.homedir();
  let defaultPath: string | undefined;
  switch (platform) {
    case "win32":
      // Windows: %LOCALAPPDATA%\Google\Chrome\User Data
      const localAppData =
        process.env.LOCALAPPDATA || path.join(homeDir, "AppData", "Local");
      defaultPath = path.join(localAppData, "Google", "Chrome", "User Data");
      break;
    case "darwin":
      // macOS: ~/Library/Application Support/Google/Chrome
      defaultPath = path.join(
        homeDir,
        "Library",
        "Application Support",
        "Google",
        "Chrome"
      );
      break;
    case "linux":
      // Linux: ~/.config/google-chrome
      defaultPath = path.join(homeDir, ".config", "google-chrome");
      break;
  }
  if (defaultPath && fs.existsSync(defaultPath)) {
    if (copyToTempDir) {
      const tempDir = os.tmpdir();
      const tempPath = path.join(tempDir, "chrome-user-data");
      if (fs.existsSync(tempPath)) {
        Log.info(`Removing existing temp directory: ${tempPath}`);
        fs.rmSync(tempPath, { recursive: true, force: true });
      }

      fs.cpSync(defaultPath, tempPath, { recursive: true });

      // Delete all Chrome locked files and directories to prevent startup conflicts.
      removeLockFiles(tempPath);

      const defaultProfilePath = path.join(tempPath, "Default");
      if (fs.existsSync(defaultProfilePath)) {
        removeLockFiles(defaultProfilePath);
      }

      Log.info(`Created clean Chrome user data directory: ${tempPath}`);
      return tempPath;
    } else {
      return defaultPath;
    }
  }
  return undefined;
}

function removeLockFiles(dirPath: string) {
  try {
    const items = fs.readdirSync(dirPath);
    items.forEach((item) => {
      const itemPath = path.join(dirPath, item);

      try {
        const stat = fs.statSync(itemPath);

        if (stat.isDirectory()) {
          removeLockFiles(itemPath);
        }

        const shouldDelete =
          item === "SingletonLock" ||
          item === "lockfile" ||
          item === "RunningChromeVersion" ||
          item === "SingletonCookie" ||
          item === "SingletonSocket" ||
          item === "chrome_debug.log" ||
          item === "LOCK" ||
          item === "LOG" ||
          item === "LOG.old" ||
          item.includes(".lock") ||
          item.includes("Lock") ||
          item.includes("LOCK") ||
          item.includes(".tmp") ||
          item.includes("Temp") ||
          item.endsWith(".pid") ||
          item.endsWith(".log") ||
          item.includes("chrome_shutdown_ms.txt") ||
          item.includes("Crashpad") ||
          (stat.isDirectory() &&
            (item.includes("CrashReports") ||
              item.includes("ShaderCache") ||
              item.includes("crashpad_database")));

        if (shouldDelete) {
          fs.rmSync(itemPath, { recursive: true, force: true });
        }
      } catch (statError) {
        if (
          item.includes("Lock") ||
          item.includes("lock") ||
          item.includes("LOCK")
        ) {
          try {
            Log.info(`Force deleting suspected lock file: ${itemPath}`);
            fs.rmSync(itemPath, { recursive: true, force: true });
          } catch (deleteError) {
            Log.warn(`Failed to force delete ${itemPath}:`, deleteError);
          }
        }
      }
    });
  } catch (error) {
    Log.warn(`Error while removing lock files from ${dirPath}:`, error);
  }
}
