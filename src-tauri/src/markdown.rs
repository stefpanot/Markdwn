//! Rendu Markdown -> HTML, avec la cartographie source qui rend le scroll
//! synchronisé et le sommaire à peu près gratuits.
//!
//! Le parti pris : UNE seule passe de parsing sur le document entier. On
//! pourrait rendre chaque bloc séparément pour connaître son offset, mais on
//! perdrait alors les définitions de liens par référence (`[a]: https://…`)
//! placées ailleurs dans le fichier. À la place, on matérialise les événements
//! puis on insère un marqueur inline avant chaque bloc de premier niveau.
//!
//! Les blocs de code clôturés (```) dont le langage est connu sont colorés
//! avec syntect pendant cette même passe : leurs événements sont avalés et
//! remplacés par un seul événement Html portant le `<pre><code>` déjà coloré.
//! Sortie en CLASSES CSS (ClassStyle::Spaced), jamais en styles en ligne :
//! le bascule clair/sombre reste purement affaire de tokens dans la webview.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use syntect::html::{ClassedHTMLGenerator, ClassStyle};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

#[derive(Debug, Serialize)]
pub struct Heading {
    /// 1 pour `#`, 2 pour `##`, …
    pub level: u8,
    pub text: String,
    pub slug: String,
    /// Ligne 1-indexée dans la source, pour le clic depuis le sommaire.
    pub line: usize,
}

#[derive(Debug, Serialize)]
pub struct Rendered {
    pub html: String,
    pub headings: Vec<Heading>,
    pub words: usize,
    /// Minutes de lecture, arrondies au supérieur, base 200 mots/min.
    pub reading_minutes: usize,
}

fn options() -> Options {
    let mut o = Options::empty();
    o.insert(Options::ENABLE_TABLES);
    o.insert(Options::ENABLE_FOOTNOTES);
    o.insert(Options::ENABLE_STRIKETHROUGH);
    o.insert(Options::ENABLE_TASKLISTS);
    o.insert(Options::ENABLE_SMART_PUNCTUATION);
    o.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    o
}

/// Table des offsets de début de ligne, pour convertir un offset d'octet en
/// numéro de ligne par recherche dichotomique.
fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    starts.extend(
        source
            .bytes()
            .enumerate()
            .filter(|(_, b)| *b == b'\n')
            .map(|(i, _)| i + 1),
    );
    starts
}

fn line_of(line_starts: &[usize], offset: usize) -> usize {
    match line_starts.binary_search(&offset) {
        Ok(i) => i + 1,
        Err(i) => i, // i = nombre de débuts de ligne <= offset
    }
}

fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_dash = false;
    for ch in text.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn level_num(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Un bloc de premier niveau reçoit un marqueur ; les tags inline non.
fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Table(_)
            | Tag::FootnoteDefinition(_)
            | Tag::HtmlBlock
            | Tag::DefinitionList
    )
}

/// Le jeu de syntaxes embarqué : décompresser le dump coûte quelques
/// millisecondes, hors de question de le payer à chaque frappe (le rendu est
/// relancé à chaque modification, débouncée mais fréquente).
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Échappement minimal identique à celui que pulldown-cmark applique au
/// contenu des blocs de code et à l'attribut `class` des `<code>`.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Colore le contenu d'un bloc clôturé et l'enveloppe dans le même
/// `<pre><code class="language-…">` que produirait pulldown-cmark : les styles
/// existants (fond, marge, mono) continuent de s'appliquer à l'identique.
fn highlight_fenced(syntax: &SyntaxReference, token: &str, code: &str) -> String {
    let mut gen =
        ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAX_SET, ClassStyle::Spaced);
    let mut ok = true;
    for line in LinesWithEndings::from(code) {
        if gen.parse_html_for_line_which_includes_newline(line).is_err() {
            ok = false;
            break;
        }
    }
    // Repli inatteignable en pratique avec les dumps précompilés : si une
    // regex fancy échouait, on servirait le code échappé sans coloration.
    let body = if ok { gen.finalize() } else { escape_html(code) };
    format!(
        "<pre><code class=\"language-{}\">{}</code></pre>",
        escape_html(token),
        body
    )
}

/// Découpe un Text sur les paires `==…==` : segments alternativement
/// ordinaires et surlignés. Règles volontairement strictes, pour ne pas
/// surligner un « a == b » ordinaire : un délimiteur ouvre seulement suivi
/// d'un non-espace, ferme seulement précédé d'un non-espace, et un ouvreur
/// sans fermeur redevient littéral. Les paires qui traverseraient un
/// formatage inline (`==a *b*==`) ne sont pas détectées : pulldown découpe
/// alors le texte en plusieurs événements. Le code inline est hors scope,
/// côté appelant, par construction.
fn split_mark(t: &str) -> Vec<(&str, bool)> {
    let is_ws = |c: Option<char>| c.is_some_and(|c| c.is_whitespace());

    let mut idx = Vec::new();
    let mut from = 0;
    while let Some(rel) = t[from..].find("==") {
        let i = from + rel;
        from = i + 2;
        idx.push(i);
    }
    // « a == b » : espaces des deux côtés, jamais un délimiteur.
    let valid: Vec<bool> = idx
        .iter()
        .map(|&i| !(is_ws(t[..i].chars().last()) && is_ws(t[i + 2..].chars().next())))
        .collect();

    let mut out: Vec<(&str, bool)> = Vec::new();
    let mut seg_start = 0usize;
    let mut k = 0usize;
    while k < idx.len() {
        if !valid[k] {
            k += 1;
            continue;
        }
        // Fermeur valide suivant : sans lui, l'ouvreur reste littéral.
        let mut j = k + 1;
        while j < idx.len() && !valid[j] {
            j += 1;
        }
        if j >= idx.len() {
            k += 1;
            continue;
        }
        out.push((&t[seg_start..idx[k]], false));
        out.push((&t[idx[k] + 2..idx[j]], true));
        seg_start = idx[j] + 2;
        k = j + 1;
    }
    out.push((&t[seg_start..], false));
    out
}

/// Nettoie le HTML rendu avant qu'il n'atteigne la webview.
///
/// pulldown-cmark laisse passer le HTML brut d'un document Markdown ; or
/// l'aperçu l'injecte tel quel ({@html} côté Svelte). Sans ce filtre, un
/// simple `<script>` ou `onerror=` dans un .md suffirait à exécuter du JS
/// dans l'application. La liste blanche d'ammonia garde tout ce que le rendu
/// produit légitimement : les classes syntect et `srcmap`, les `id` d'ancre
/// des titres, les `data-line` de la cartographie source, les cases à cocher
/// des tasklists et les URLs relatives que le front résout ensuite en Rust.
fn sanitize(html: &str) -> String {
    ammonia::Builder::default()
        // Les attributs que NOTRE rendu émet et dont la webview a besoin.
        .add_generic_attributes(["class", "id", "data-line"])
        // Tasklists (ENABLE_TASKLISTS) : `<input disabled type="checkbox">`.
        .add_tags(["input", "mark"])
        .add_tag_attributes("input", ["type", "checked", "disabled"])
        // Les href/src relatifs (`./autre.md`, `images/x.png`) doivent
        // survivre : c'est le front qui les résout et les réécrit.
        .url_relative(ammonia::UrlRelative::PassThrough)
        // Images data: URI (resolve_asset les laisse telles quelles) et src
        // déjà réécrits vers le protocole asset: de Tauri.
        .add_url_schemes(["data", "asset"])
        .clean(html)
        .to_string()
}

pub fn render(source: &str) -> Rendered {
    let starts = line_starts(source);
    let mut parser = Parser::new_ext(source, options()).into_offset_iter();

    let mut events: Vec<Event> = Vec::new();
    let mut headings: Vec<Heading> = Vec::new();
    let mut words = 0usize;

    // Profondeur d'imbrication : on ne marque que les blocs de niveau 0.
    let mut depth: i32 = 0;
    // Titre en cours de collecte, s'il y en a un.
    let mut current_heading: Option<(u8, usize, String)> = None;
    // Index dans `events` de chaque Start(Heading), pour y injecter l'id une
    // fois le texte du titre connu — on ne peut pas le faire à l'aller.
    let mut heading_at: Vec<usize> = Vec::new();
    let mut slug_seen: HashMap<String, usize> = HashMap::new();

    while let Some((event, range)) = parser.next() {
        // Bloc clôturé dont le langage est connu de syntect : on avale les
        // événements jusqu'à la fin du bloc et on les remplace par le HTML
        // coloré. Le Start/End étant consommés ensemble, la profondeur reste
        // équilibrée ; les blocs indentés et les langages inconnus passent au
        // flux normal, inchangé.
        if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) = &event {
            let token = lang.split_whitespace().next().unwrap_or("");
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_token(token) {
                if depth == 0 {
                    let line = line_of(&starts, range.start);
                    events.push(Event::Html(
                        format!("<span class=\"srcmap\" data-line=\"{line}\"></span>").into(),
                    ));
                }
                let mut code = String::new();
                for (inner, _) in parser.by_ref() {
                    match inner {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(t) | Event::Code(t) => {
                            words += t.split_whitespace().count();
                            code.push_str(&t);
                        }
                        _ => {}
                    }
                }
                events.push(Event::Html(highlight_fenced(syntax, token, &code).into()));
                continue;
            }
        }
        match &event {
            Event::Start(tag) => {
                if is_block_tag(tag) && depth == 0 {
                    let line = line_of(&starts, range.start);
                    events.push(Event::Html(
                        format!("<span class=\"srcmap\" data-line=\"{line}\"></span>").into(),
                    ));
                }
                if let Tag::Heading { level, .. } = tag {
                    current_heading =
                        Some((level_num(*level), line_of(&starts, range.start), String::new()));
                    heading_at.push(events.len());
                }
                depth += 1;
            }
            Event::End(end) => {
                depth -= 1;
                if matches!(end, TagEnd::Heading(_)) {
                    if let Some((level, line, text)) = current_heading.take() {
                        let base = slugify(&text);
                        // Deux titres homonymes ne peuvent pas partager une ancre.
                        let n = slug_seen.entry(base.clone()).or_insert(0);
                        *n += 1;
                        let slug = if *n == 1 { base } else { format!("{base}-{n}") };
                        headings.push(Heading { level, text, slug, line });
                    }
                }
            }
            Event::Text(t) => {
                if t.contains("==") {
                    // `==surligné==` n'existe pas dans CommonMark : on l'émet
                    // comme HTML inline, dont le rendu est assuré par <mark>.
                    // Comptage et sommaire portent sur le texte débarrassé des
                    // délimiteurs de paires valides.
                    let segs = split_mark(t);
                    let clean: String = segs.iter().map(|(s, _)| *s).collect();
                    words += clean.split_whitespace().count();
                    if let Some((_, _, text)) = current_heading.as_mut() {
                        text.push_str(&clean);
                    }
                    for (seg, marked) in segs {
                        if seg.is_empty() {
                            continue;
                        }
                        if marked {
                            events.push(Event::Html("<mark>".into()));
                        }
                        // Owned : un segment emprunté à `event` ne survivrait
                        // pas à l'itération (events vit plus longtemps).
                        events.push(Event::Text(seg.to_string().into()));
                        if marked {
                            events.push(Event::Html("</mark>".into()));
                        }
                    }
                    // Déjà poussés pièce par pièce : le push final ne s'applique pas.
                    continue;
                }
                words += t.split_whitespace().count();
                if let Some((_, _, text)) = current_heading.as_mut() {
                    text.push_str(t);
                }
            }
            Event::Code(t) => {
                words += t.split_whitespace().count();
                if let Some((_, _, text)) = current_heading.as_mut() {
                    text.push_str(t);
                }
            }
            _ => {}
        }
        events.push(event);
    }

    // Les titres portent leur ancre : c'est ce qui rend `#slug` cliquable et
    // les liens profonds entre documents possibles.
    for (idx, heading) in heading_at.iter().zip(headings.iter()) {
        if let Some(Event::Start(Tag::Heading { id, .. })) = events.get_mut(*idx) {
            *id = Some(heading.slug.clone().into());
        }
    }

    let mut html = String::with_capacity(source.len() * 3 / 2);
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    let html = sanitize(&html);

    Rendered {
        html,
        headings,
        words,
        reading_minutes: words.div_ceil(200).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_each_top_level_block_with_its_line() {
        let r = render("# Titre\n\nUn paragraphe.\n");
        assert!(r.html.contains("data-line=\"1\""));
        assert!(r.html.contains("data-line=\"3\""));
    }

    #[test]
    fn does_not_mark_nested_blocks() {
        // Le paragraphe est dans la citation : un seul marqueur, pas deux.
        let r = render("> Cité\n");
        assert_eq!(r.html.matches("class=\"srcmap\"").count(), 1);
    }

    #[test]
    fn collects_headings_with_slug_and_line() {
        let r = render("# Un\n\ntexte\n\n## Deux mots\n");
        assert_eq!(r.headings.len(), 2);
        assert_eq!(r.headings[0].level, 1);
        assert_eq!(r.headings[1].slug, "deux-mots");
        assert_eq!(r.headings[1].line, 5);
    }

    #[test]
    fn resolves_reference_links_defined_elsewhere() {
        // C'est précisément ce qu'un rendu bloc-par-bloc casserait.
        let r = render("Voir [la doc][d].\n\n[d]: https://exemple.test\n");
        assert!(r.html.contains("href=\"https://exemple.test\""));
    }

    #[test]
    fn counts_words_and_reading_time() {
        let r = render("un deux trois\n");
        assert_eq!(r.words, 3);
        assert_eq!(r.reading_minutes, 1);
    }

    #[test]
    fn renders_tables() {
        let r = render("| a | b |\n| - | - |\n| 1 | 2 |\n");
        assert!(r.html.contains("<table>"));
    }

    #[test]
    fn headings_carry_their_anchor() {
        let r = render("## Port registry\n");
        assert!(r.html.contains("id=\"port-registry\""));
    }

    #[test]
    fn homonymous_headings_get_distinct_anchors() {
        let r = render("## Notes\n\ntexte\n\n## Notes\n");
        assert_eq!(r.headings[0].slug, "notes");
        assert_eq!(r.headings[1].slug, "notes-2");
        assert!(r.html.contains("id=\"notes\""));
        assert!(r.html.contains("id=\"notes-2\""));
    }

    #[test]
    fn mark_pairs_become_highlight_html() {
        let r = render("Un passage ==très important== ici.\n");
        assert!(r.html.contains("<mark>très important</mark>"));
        // Le texte reste compté dans les mots (délimiteurs exclus).
        assert_eq!(r.words, 5);
    }

    #[test]
    fn mark_leaves_heading_text_clean() {
        let r = render("## Réunion ==du lundi==\n");
        assert_eq!(r.headings[0].text, "Réunion du lundi");
        assert!(r.html.contains("<mark>du lundi</mark>"));
    }

    #[test]
    fn unpaired_mark_stays_literal() {
        let r = render("Un ==seul délimiteur.\n");
        assert!(!r.html.contains("<mark>"));
        assert!(r.html.contains("==seul"));
    }

    #[test]
    fn code_inline_is_never_marked() {
        let r = render("`==pas de surligné==`\n");
        assert!(!r.html.contains("<mark>"));
    }

    #[test]
    fn highlights_fenced_code_with_known_language() {
        let r = render("```rust\nfn main() { let x = 1; }\n```\n");
        // Même enveloppe qu'avant, pour garder les styles existants…
        assert!(r.html.contains("<pre><code class=\"language-rust\">"));
        // …mais le contenu est découpé en spans de classes de scopes syntect.
        // `source rust` est le scope racine : il est toujours émis.
        assert!(r.html.contains("<span class=\"source rust\">"));
        assert!(r.html.matches("<span class=\"").count() > 1);
        // Sortie en classes, jamais en styles en ligne.
        assert!(!r.html.contains("style="));
    }

    #[test]
    fn unknown_language_falls_back_to_plain_code() {
        let r = render("```zz-nope\nnuqneH <qaH>\n```\n");
        assert!(r.html.contains("<pre><code class=\"language-zz-nope\">"));
        assert!(r.html.contains("nuqneH &lt;qaH&gt;"));
        assert!(!r.html.contains("<span class=\"source"));
    }

    #[test]
    fn bare_fence_falls_back_to_plain_code() {
        let r = render("```\nlet x = 1;\n```\n");
        assert!(r.html.contains("<pre><code>let x = 1;"));
        assert!(!r.html.contains("<span class=\"source"));
    }

    #[test]
    fn srcmap_markers_still_wrap_code_blocks() {
        let r = render("Avant.\n\n```rust\nfn f() {}\n```\n\nAprès.\n");
        // Le marqueur du bloc coloré porte la ligne de sa clôture ouvrante.
        assert!(r.html.contains("data-line=\"3\""));
        assert_eq!(r.html.matches("class=\"srcmap\"").count(), 3);
    }

    #[test]
    fn indented_code_blocks_are_left_alone() {
        let r = render("    let x = 1;\n");
        assert!(r.html.contains("<pre><code>let x = 1;"));
        assert!(!r.html.contains("<span class=\"source"));
    }

    #[test]
    fn highlighted_code_is_still_counted_as_words() {
        // Le code comptait avant la coloration ; l'interception le compte
        // toujours (« let », « a », « = », « 1; »).
        let r = render("```rust\nlet a = 1;\n```\n");
        assert_eq!(r.words, 4);
    }

    // --- Sanification : le HTML d'un document ne doit rien exécuter. ---

    #[test]
    fn raw_script_is_stripped() {
        let r = render("# Titre\n\n<script>alert(1)</script>\n");
        assert!(!r.html.contains("<script"));
        assert!(!r.html.contains("alert(1)"));
        // Le reste du document est rendu normalement.
        assert!(r.html.contains("Titre"));
    }

    #[test]
    fn inline_event_handlers_are_stripped() {
        // pulldown laisse passer les <img> en HTML brut ; le front les repère
        // ensuite par leur src pour les résoudre. onerror doit disparaître.
        let r = render("<img src=\"x.png\" onerror=\"alert(1)\">\n");
        assert!(r.html.contains("src=\"x.png\""));
        assert!(!r.html.contains("onerror"));
    }

    #[test]
    fn javascript_links_are_stripped() {
        let r = render("[cliquer](javascript:alert(1))\n");
        assert!(!r.html.contains("javascript:"));
    }

    #[test]
    fn tasklist_checkboxes_survive() {
        let r = render("- [x] fait\n- [ ] à faire\n");
        assert_eq!(r.html.matches("type=\"checkbox\"").count(), 2);
        assert!(r.html.contains("checked"));
        assert!(r.html.contains("disabled"));
    }

    #[test]
    fn relative_links_and_images_survive() {
        // Le front résout ces src/href en Rust : ils doivent traverser la
        // sanification (ammonia refuse par défaut les URLs relatives).
        let r = render("[doc](./autre.md)\n\n![img](images/x.png)\n");
        assert!(r.html.contains("href=\"./autre.md\""));
        assert!(r.html.contains("src=\"images/x.png\""));
    }

    #[test]
    fn markup_produced_by_the_renderer_survives() {
        let r = render("# ==Titre==\n\nTexte avec `du code`.\n");
        // ancre, surlignage et classes syntect : tout ce que le front
        // consomme doit être encore là après le passage dans ammonia.
        assert!(r.html.contains("id=\"titre\""));
        assert!(r.html.contains("<mark>Titre</mark>"));
        assert!(r.html.contains("srcmap"));
        assert!(r.html.contains("data-line=\"1\""));
    }
}
