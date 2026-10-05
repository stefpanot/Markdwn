export type FormatKind =
  | "bold"
  | "italic"
  | "code"
  | "codeblock"
  | "strike"
  | "mark"
  | "link"
  | "list"
  | "quote";

/** Actions du menu d'édition (répliques du menu contextuel). */
export type EditAction = "cut" | "copy" | "paste" | "selectAll";
