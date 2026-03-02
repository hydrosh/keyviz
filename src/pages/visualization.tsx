import { KeyOverlay } from "@/components/key-overlay";
import { MouseOverlay } from "@/components/mouse-overlay";
import { KEY_EVENT_STORE, KeyEventStore, useKeyEvent } from "@/stores/key_event";
import { KEY_STYLE_STORE, KeyStyleStore, useKeyStyle } from '@/stores/key_style';
import { listenForUpdates } from '@/stores/sync';
import { EventPayload } from "@/types/event";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, primaryMonitor } from "@tauri-apps/api/window";
import { useEffect, useState, } from "react";

export function Visualization() {
  const monitor = useKeyStyle((state) => state.appearance.monitor);
  const onEvent = useKeyEvent((state) => state.onEvent);
  const tick = useKeyEvent((state) => state.tick);

  // listening for input events
  const [isListening, setIsListening] = useState(true);

  // Per-window offset used to convert absolute mouse coordinates to
  // window-relative coordinates in multi-monitor mode.
  const [windowOffset, setWindowOffset] = useState({ x: 0, y: 0 });

  useEffect(() => {
    const unlistenPromises = [
      // ───────────── input event listener ─────────────
      listen<EventPayload>("input-event", (event) => onEvent(event.payload)),
      // ───────────── store sync ─────────────
      listenForUpdates<KeyEventStore>(KEY_EVENT_STORE, useKeyEvent.setState),
      listenForUpdates<KeyStyleStore>(KEY_STYLE_STORE, useKeyStyle.setState),
      // ───────────── settings window open/close ─────────────
      listen<boolean>("settings-window", (event) => {
        useKeyEvent.setState({ settingsOpen: event.payload });
      }),
      // ───────────── listener toggle ─────────────
      listen<boolean>("listening-toggle", (event) => setIsListening(event.payload)),
    ];
    const id = setInterval(tick, 250);

    return () => {
      clearInterval(id);
      unlistenPromises.forEach((p) => p.then((f) => f()));
    };
  }, []);

  // Only the main window is responsible for invoking the monitor setup command.
  // Overlay windows created for multi-monitor mode must not call this, otherwise
  // they would trigger a new round of overlay-window creation.
  useEffect(() => {
    const currentWindow = getCurrentWindow();
    if (currentWindow.label !== "main") return;

    const set_monitor = async () => {
      let monitorName = monitor;
      if (!monitorName) {
        const primary = await primaryMonitor();
        monitorName = primary?.name ?? "";
      }
      if (!monitorName) return;
      await invoke("set_main_window_monitor", { monitorName });
    }
    set_monitor();
  }, [monitor]);

  // In multi-monitor mode the backend emits raw screen coordinates (no offset
  // subtracted). Each window computes its own offset so the mouse indicator is
  // positioned correctly relative to that window's top-left corner.
  useEffect(() => {
    if (monitor !== "all") {
      setWindowOffset({ x: 0, y: 0 });
      return;
    }

    const currentWindow = getCurrentWindow();
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    const updateOffset = async () => {
      const pos = await currentWindow.outerPosition();
      setWindowOffset({ x: pos.x, y: pos.y });
    };
    updateOffset();

    currentWindow.onMoved(() => updateOffset()).then(fn => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [monitor]);

  if (!isListening) return null;

  return <div className="w-screen h-screen relative overflow-hidden">
    <MouseOverlay windowOffset={windowOffset} />
    <KeyOverlay />
  </div>;
}