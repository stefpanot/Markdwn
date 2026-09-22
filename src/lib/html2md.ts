/** Convertit du HTML de presse-papier en Markdown.
 *
 * Couvre les balises qu'on trouve dans le contenu copié depuis un éditeur web
 * ou un traitement de texte : titres, paragraphes, gras/italique/barré/
 * surligné, liens, images, listes (y compris imbriquées), citations, code et
 * séparateurs. Le reste retombe sur son texte — on ne perd jamais de contenu.
 */

function inlineOf(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) return node.textContent ?? "";
  if (node.nodeType !== Node.ELEMENT_NODE) return "";

  const el = node as Element;
  const inner = () => Array.from(el.childNodes).map(inlineOf).join("");

  switch (el.tagName.toLowerCase()) {
    case "strong":
    case "b":
      return `**${inner().trim()}**`;
    case "em":
    case "i":
      return `*${inner().trim()}*`;
    case "del":
    case "s":
    case "strike":
      return `~~${inner().trim()}~~`;
    case "mark":
      return `==${inner().trim()}==`;
    case "code": {
      const text = el.textContent ?? "";
      const fence = text.includes("`") ? "``" : "`";
      return `${fence}${text}${fence}`;
    }
    case "a": {
      const href = el.getAttribute("href") ?? "";
      const text = inner().trim();
      /* Schémas dangereux : la normalisation (trim + casse) compte, car
         «  javascript: » ou « JaVaScRiPt: » passeraient le filtre brut.
         data: et vbscript: rejoignent javascript: (alerte CodeQL
         « Incomplete URL scheme check »). */
      const normalized = href.trim().toLowerCase();
      const dangerous = ["javascript:", "data:", "vbscript:"].some((s) => normalized.startsWith(s));
      if (!normalized || dangerous) return text;
      if (!text) return "";
      return `[${text}](${href})`;
    }
    case "img": {
      const src = el.getAttribute("src") ?? "";
      const alt = el.getAttribute("alt") ?? "";
      if (!src) return "";
      return `![${alt}](${src})`;
    }
    case "br":
      return "\n";
    case "span":
    case "font":
    case "u":
      return inner();
    default:
      // td/th en mode inline (cellules d'un tableau simplifié) : leur texte.
      if (el.tagName === "TD" || el.tagName === "TH") return ` ${inner().trim()} |`;
      return inner();
  }
}

function listOf(list: Element, depth: number, ordered: boolean): string {
  const indent = "  ".repeat(depth);
  const lines: string[] = [];
  let n = 0;
  for (const li of Array.from(list.children)) {
    if (li.tagName.toLowerCase() !== "li") continue;
    n += 1;
    const marker = ordered ? `${n}.` : "-";
    const parts: string[] = [];
    const nested: string[] = [];
    for (const child of Array.from(li.childNodes)) {
      const el = child as Element;
      if (child.nodeType === Node.ELEMENT_NODE && /^[uo]l$/i.test(el.tagName)) {
        nested.push(listOf(el, depth + 1, el.tagName.toLowerCase() === "ol"));
      } else if (child.nodeType === Node.ELEMENT_NODE && /^p$/i.test(el.tagName)) {
        // <li><p>…</p></li> : le paragraphe devient la ligne de l'item.
        parts.push(inlineOf(child).trim());
      } else {
        parts.push(inlineOf(child).trim());
      }
    }
    lines.push(`${indent}${marker} ${parts.filter(Boolean).join(" ")}`.trimEnd());
    for (const sub of nested) lines.push(sub);
  }
  return lines.join("\n");
}

function blockOf(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) {
    return (node.textContent ?? "").trim();
  }
  if (node.nodeType !== Node.ELEMENT_NODE) return "";

  const el = node as Element;
  const tag = el.tagName.toLowerCase();

  if (/^h[1-6]$/.test(tag)) {
    const level = Number(tag[1]);
    return `${"#".repeat(level)} ${inlineOf(el).trim()}`;
  }
  if (tag === "pre") {
    const codeEl = el.querySelector("code");
    const cls = codeEl?.className.match(/language-([\w-]+)/);
    const lang = cls?.[1] ?? "";
    const text = (codeEl ?? el).textContent?.replace(/\n$/, "") ?? "";
    return `\`\`\`${lang}\n${text}\n\`\`\``;
  }
  if (tag === "ul" || tag === "ol") {
    return listOf(el, 0, tag === "ol");
  }
  if (tag === "blockquote") {
    const inner = blocksOf(el).join("\n\n");
    return inner
      .split("\n")
      .map((l) => (l ? `> ${l}` : ">"))
      .join("\n");
  }
  if (tag === "hr") return "---";
  if (tag === "table") {
    // Tableau simplifié : une ligne par rangée, cellules séparées par « | ».
    const rows = Array.from(el.querySelectorAll("tr"))
      .map((tr) => `|${Array.from(tr.children).map((c) => inlineOf(c).trim()).join("")}`)
      .filter((r) => r.length > 1);
    return rows.join("\n");
  }
  // p, div et tout autre conteneur : leur contenu inline.
  return inlineOf(el).trim();
}

function blocksOf(el: Element): string[] {
  const out: string[] = [];
  for (const child of Array.from(el.childNodes)) {
    const text = blockOf(child);
    if (text) out.push(text);
  }
  return out;
}

export function htmlToMarkdown(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const body = doc.body;
  if (!body) return "";
  return `${blocksOf(body).join("\n\n").replace(/\n{3,}/g, "\n\n").trim()}\n`;
}
