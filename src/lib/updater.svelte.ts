// Over-the-air update state. A single shared store drives the global update
// banner so the app can surface and apply updates while the user keeps working.
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdatePhase =
  | "idle" // nothing to show
  | "checking" // querying the endpoint
  | "available" // an update exists, not yet downloaded
  | "downloading" // pulling the package
  | "ready" // downloaded + installed; waiting for restart
  | "uptodate" // manual check found nothing new
  | "error"; // a manual check/download failed

class UpdaterState {
  phase = $state<UpdatePhase>("idle");
  version = $state<string | null>(null);
  notes = $state<string | null>(null);
  error = $state<string | null>(null);
  downloaded = $state(0);
  total = $state(0);
  dismissed = $state(false);

  // The pending update handle (not reactive — only used imperatively).
  #update: Update | null = null;

  get progress(): number {
    return this.total > 0
      ? Math.min(100, Math.round((this.downloaded / this.total) * 100))
      : 0;
  }

  // True while a long operation is in flight (used to disable buttons).
  get busy(): boolean {
    return this.phase === "checking" || this.phase === "downloading";
  }

  /**
   * Check the release endpoint for a newer version.
   * `manual` controls the "nothing found" / error UX: a background check stays
   * silent, a manual check reports the result so the user gets feedback.
   */
  async check(manual = false): Promise<void> {
    if (this.busy) return;
    this.phase = "checking";
    this.error = null;
    try {
      const update = await check();
      if (update) {
        this.#update = update;
        this.version = update.version;
        this.notes = update.body ?? null;
        this.dismissed = false;
        this.phase = "available";
      } else {
        this.phase = manual ? "uptodate" : "idle";
      }
    } catch (e) {
      this.error = String(e);
      this.phase = manual ? "error" : "idle";
    }
  }

  /** Download + install the pending update, streaming progress to the UI. */
  async downloadAndInstall(): Promise<void> {
    if (!this.#update) return;
    this.phase = "downloading";
    this.downloaded = 0;
    this.total = 0;
    this.error = null;
    try {
      await this.#update.downloadAndInstall((ev) => {
        switch (ev.event) {
          case "Started":
            this.total = ev.data.contentLength ?? 0;
            break;
          case "Progress":
            this.downloaded += ev.data.chunkLength;
            break;
          case "Finished":
            this.downloaded = this.total;
            break;
        }
      });
      this.phase = "ready";
    } catch (e) {
      this.error = String(e);
      this.phase = "error";
    }
  }

  /** Relaunch into the freshly installed version. */
  async restart(): Promise<void> {
    await relaunch();
  }

  /** Hide the banner for the current update without applying it. */
  dismiss(): void {
    this.dismissed = true;
    if (this.phase === "uptodate" || this.phase === "error") {
      this.phase = "idle";
    }
  }
}

export const updater = new UpdaterState();
