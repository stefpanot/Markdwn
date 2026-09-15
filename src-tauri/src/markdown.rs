//! Rendu Markdown -> HTML, avec la cartographie source qui rend le scroll
//! synchronisé et le sommaire à peu près gratuits.
//!
//! Le parti pris : UNE seule passe de parsing sur le document entier. On
//! pourrait rendre chaque bloc séparément pour connaître son offset, mais on
//! perdrait alors les définitions de liens par référence (`[a]: https://…`)
//! placées ailleurs dans le fichier. À la place, on matérialise les événements
//! puis on insère un marqueur inline avant chaque bloc de premier niveau.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use std::collections::HashMap;

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

pub fn render(source: &str) -> Rendered {
    let starts = line_starts(source);
    let parser = Parser::new_ext(source, options()).into_offset_iter();

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

    for (event, range) in parser {
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
            Event::Text(t) | Event::Code(t) => {
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
}
