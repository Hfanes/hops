import { invoke } from "@tauri-apps/api/core";
import type { OpenUrlRequest, PickerSession } from "../types";

export function getPickerState(): Promise<PickerSession | null> {
  return invoke<PickerSession | null>("get_picker_state");
}

export function hidePickerWindow(): Promise<void> {
  return invoke<void>("hide_picker_window");
}

export function openUrl(request: OpenUrlRequest): Promise<void> {
  return invoke<void>("open_url", { request });
}
