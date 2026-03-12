import { vi } from "vitest";

type InvokeHandler = (cmd: string, args?: Record<string, unknown>) => unknown;

let invokeHandler: InvokeHandler = () => undefined;

export function setInvokeHandler(handler: InvokeHandler) {
  invokeHandler = handler;
}

export function resetInvokeHandler() {
  invokeHandler = () => undefined;
}

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd: string, args?: Record<string, unknown>) =>
    Promise.resolve(invokeHandler(cmd, args)),
  ),
}));

vi.mock("@tauri-apps/api/event", () => {
  const listeners: Map<string, Set<(event: { payload: unknown }) => void>> =
    new Map();
  return {
    listen: vi.fn(
      (event: string, handler: (event: { payload: unknown }) => void) => {
        if (!listeners.has(event)) {
          listeners.set(event, new Set());
        }
        listeners.get(event)!.add(handler);
        return Promise.resolve(() => {
          listeners.get(event)?.delete(handler);
        });
      },
    ),
    emit: vi.fn((event: string, payload: unknown) => {
      listeners.get(event)?.forEach((handler) => handler({ payload }));
      return Promise.resolve();
    }),
    __listeners: listeners,
  };
});
