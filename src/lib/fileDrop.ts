import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";

export interface DropPosition {
  x: number;
  y: number;
}

export interface FileDrop {
  paths: string[];
  position?: DropPosition;
}

interface FileDropHandlers {
  onEnter?: (paths: string[]) => void;
  onOver?: (position?: DropPosition) => void;
  onLeave?: () => void;
  onDrop: (drop: FileDrop) => void | Promise<void>;
}

export function splitPdfPaths(paths: string[]) {
  const pdfs: string[] = [];
  const rejected: string[] = [];

  for (const path of paths) {
    if (path.toLowerCase().endsWith(".pdf")) {
      pdfs.push(path);
    } else {
      rejected.push(path);
    }
  }

  return { pdfs, rejected };
}

export function fileNameFromPath(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export async function listenForFileDrops(
  handlers: FileDropHandlers,
): Promise<UnlistenFn> {
  try {
    return await getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;

      if (payload.type === "enter") {
        handlers.onEnter?.(payload.paths);
        return;
      }

      if (payload.type === "over") {
        handlers.onOver?.(payload.position);
        return;
      }

      if (payload.type === "leave") {
        handlers.onLeave?.();
        return;
      }

      void handlers.onDrop({
        paths: payload.paths,
        position: payload.position,
      });
    });
  } catch {
    return () => {};
  }
}
