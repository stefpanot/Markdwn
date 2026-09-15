import { invoke } from "@tauri-apps/api/core";

export interface Heading {
  level: number;
  text: string;
  slug: string;
  line: number;
}

export interface Rendered {
  html: string;
  headings: Heading[];
  words: number;
  reading_minutes: number;
}

export interface Document {
  path: string;
  name: string;
  content: string;
}

export interface DirEntryInfo {
  path: string;
  name: string;
  is_dir: boolean;
}

/** Tout le parsing Markdown vit en Rust : le front ne fait que patcher le DOM. */
export const renderMarkdown = (source: string) =>
  invoke<Rendered>("render_markdown", { source });

export const readDocument = (path: string) =>
  invoke<Document>("read_document", { path });

export const writeDocument = (path: string, content: string) =>
  invoke<void>("write_document", { path, content });

export const listDir = (path: string) =>
  invoke<DirEntryInfo[]>("list_dir", { path });

/** Liste plate des .md, dans l'ordre de l'arborescence affichée : c'est ce qui
    donne « fichier suivant / précédent » sans surprise. */
export const listMarkdownTree = (path: string) =>
  invoke<string[]>("list_markdown_tree", { path });

/** Miroir exact de `config::Config` côté Rust (serde en camelCase). */
export interface AppConfig {
  schemaVersion: number;
  theme: "dark" | "light";
  mode: "read" | "split" | "zen";
  readingSize: number;
  readingWidth: "centered" | "full";
  sidebarVisible: boolean;
  syncScroll: boolean;
  lastFolder: string;
  restoreLastFolder: boolean;
}

export interface LoadedConfig {
  config: AppConfig;
  /** Non nul si le fichier a dû être écarté : à montrer à l'utilisateur. */
  warning: string | null;
  path: string;
}

/** Résolue depuis Cargo.toml, seule source de vérité. */
export const appVersion = () => invoke<string>("app_version");

export const loadConfig = () => invoke<LoadedConfig>("load_config");
export const saveConfig = (config: AppConfig) => invoke<void>("save_config", { config });

export interface ResolvedLink {
  path: string;
  exists: boolean;
  is_markdown: boolean;
}

/** Résout un lien relatif depuis le document courant. La normalisation des
    `..` et le décodage percent vivent en Rust. */
export const resolveLink = (from: string, href: string) =>
  invoke<ResolvedLink>("resolve_link", { from, href });
