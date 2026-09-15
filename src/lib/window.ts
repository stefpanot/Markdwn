import { getCurrentWindow } from "@tauri-apps/api/window";

/** Le chrome est dessiné par nous (decorations: false), donc les boutons de
    fenêtre doivent appeler l'API. Hors Tauri (vite dev nu) on ne casse pas. */
async function win() {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export const minimise = async () => (await win())?.minimize();
export const toggleMaximise = async () => (await win())?.toggleMaximize();
export const closeWindow = async () => (await win())?.close();

export type Edge =
  | "North"
  | "South"
  | "East"
  | "West"
  | "NorthEast"
  | "NorthWest"
  | "SouthEast"
  | "SouthWest";

/** `decorations: false` supprime aussi les bordures de redimensionnement
    natives ; on les redonne via des poignées invisibles. Sans effet quand la
    fenêtre est agrandie, sinon on la restaurerait par accident. */
export async function startResize(edge: Edge) {
  const w = await win();
  if (!w) return;
  if (await w.isMaximized()) return;
  await w.startResizeDragging(edge);
}
