//! Moteur de recherche / remplacement du document courant.
//!
//! Socle de la future recherche multi-fichiers (Piste 3) : tout ce qui touche
//! au texte vit ici, pas dans la webview. Deux options seulement — casse et
//! mot entier — la regex reste hors scope pour l'instant.
//!
//! Les positions renvoyées sont en UNITÉS UTF-16 : ce sont les unités des
//! strings JavaScript ET de CodeMirror, donc le front les consomme telles
//! quelles, sans conversion.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Match {
    /// Début et longueur en unités UTF-16.
    pub index: usize,
    pub len: usize,
    /// 1-based, en unités UTF-16 depuis le début de ligne. Pour l'affichage
    /// et la future recherche multi-fichiers.
    pub line: usize,
    pub col: usize,
    /// Texte effectivement matché : le front vérifie qu'il est encore là
    /// avant de remplacer (garde contre les offsets périmés).
    pub text: String,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Positions de départ (en INDEX DE CARACTÈRES, pas d'UTF-16) de toutes les
/// occurrences, non chevauchantes : après un succès on saute toute la query.
fn match_starts(text: &[char], q: &[char], opts: SearchOptions) -> Vec<usize> {
    let n = q.len();
    if n == 0 || n > text.len() {
        return Vec::new();
    }

    // La comparaison se fait sur la forme repliée ; la longueur REMPLACÉE
    // vient toujours du texte ('İ'.to_lowercase() vaut « i̇ », deux chars).
    let folded: Vec<String> = q.iter().map(|c| c.to_lowercase().collect()).collect();

    let mut out = Vec::new();
    let mut i = 0;
    while i + n <= text.len() {
        let mut hit = true;
        for j in 0..n {
            let c = text[i + j];
            if if opts.case_sensitive {
                c != q[j]
            } else {
                c.to_lowercase().ne(folded[j].chars())
            } {
                hit = false;
                break;
            }
        }
        if hit && opts.whole_word {
            let before = i > 0 && is_word_char(text[i - 1]);
            let after = i + n < text.len() && is_word_char(text[i + n]);
            hit = !before && !after;
        }
        if hit {
            out.push(i);
            i += n;
        } else {
            i += 1;
        }
    }
    out
}

pub fn find_matches(source: &str, query: &str, opts: SearchOptions) -> Vec<Match> {
    let text: Vec<char> = source.chars().collect();
    let q: Vec<char> = query.chars().collect();
    if q.is_empty() {
        return Vec::new();
    }
    let starts = match_starts(&text, &q, opts);
    if starts.is_empty() {
        return Vec::new();
    }

    // Tables de conversion position-en-caractères -> UTF-16 / ligne / colonne.
    // Un caractère astral (😀) pèse 2 unités UTF-16, d'où les deux compteurs.
    let mut utf16 = Vec::with_capacity(text.len() + 1);
    let mut lines = Vec::with_capacity(text.len() + 1);
    let mut cols = Vec::with_capacity(text.len() + 1);
    let (mut off, mut line, mut col) = (0usize, 1usize, 0usize);
    for &c in &text {
        utf16.push(off);
        lines.push(line);
        cols.push(col);
        let w = c.len_utf16();
        off += w;
        col += w;
        if c == '\n' {
            line += 1;
            col = 0;
        }
    }
    utf16.push(off);
    lines.push(line);
    cols.push(col);

    let n = q.len();
    starts
        .iter()
        .map(|&i| Match {
            index: utf16[i],
            len: utf16[i + n] - utf16[i],
            line: lines[i],
            col: cols[i] + 1,
            text: text[i..i + n].iter().collect(),
        })
        .collect()
}

pub fn replace_all(source: &str, query: &str, replacement: &str, opts: SearchOptions) -> String {
    let text: Vec<char> = source.chars().collect();
    let q: Vec<char> = query.chars().collect();
    if q.is_empty() {
        return source.to_string();
    }
    let starts = match_starts(&text, &q, opts);
    if starts.is_empty() {
        return source.to_string();
    }

    let n = q.len();
    let mut out = String::with_capacity(source.len());
    let mut pos = 0usize;
    for &s in &starts {
        out.extend(text[pos..s].iter());
        out.push_str(replacement);
        pos = s + n;
    }
    out.extend(text[pos..].iter());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTS: SearchOptions = SearchOptions {
        case_sensitive: true,
        whole_word: false,
    };

    fn indices(source: &str, query: &str, opts: SearchOptions) -> Vec<usize> {
        find_matches(source, query, opts).into_iter().map(|m| m.index).collect()
    }

    #[test]
    fn finds_occurrences_with_line_and_col() {
        let m = find_matches("un deux\nun trois", "un", OPTS);
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].index, m[0].line, m[0].col), (0, 1, 1));
        assert_eq!((m[1].index, m[1].line, m[1].col), (8, 2, 1));
    }

    #[test]
    fn empty_query_finds_nothing() {
        assert!(find_matches("texte", "", OPTS).is_empty());
        assert_eq!(replace_all("texte", "", "x", OPTS), "texte");
    }

    #[test]
    fn honours_case_sensitivity() {
        assert_eq!(indices("Rust rust RUST", "rust", OPTS), vec![5]);
        let ci = SearchOptions {
            case_sensitive: false,
            whole_word: false,
        };
        assert_eq!(indices("Rust rust RUST", "rust", ci), vec![0, 5, 10]);
    }

    #[test]
    fn whole_word_checks_alphanumeric_boundaries() {
        let ww = SearchOptions {
            case_sensitive: true,
            whole_word: true,
        };
        // « chai » n'est mot entier ni dans « chaise » ni dans « chair ».
        assert_eq!(indices("chaise chair", "chai", ww), Vec::<usize>::new());
        assert_eq!(indices("chaise chair", "chaise", ww), vec![0]);
        assert_eq!(indices("chaise chair", "chair", ww), vec![7]);
        // les underscores comptent comme caractère de mot
        assert_eq!(indices("un_chai", "chai", ww), Vec::<usize>::new());
    }

    #[test]
    fn matches_do_not_overlap() {
        assert_eq!(indices("aaaa", "aa", OPTS), vec![0, 2]);
        assert_eq!(indices("aaa", "aa", OPTS), vec![0]);
    }

    #[test]
    fn offsets_are_utf16_code_units() {
        // 😀 pèse 2 unités UTF-16 : « b » démarre donc à 3, pas à 2.
        let m = find_matches("a😀b", "b", OPTS);
        assert_eq!(m[0].index, 3);
        assert_eq!(m[0].len, 1);
        // longueur d'occurrence : un astral matché vaut 2 unités.
        let m = find_matches("x😀y", "😀", OPTS);
        assert_eq!((m[0].index, m[0].len), (1, 2));
        // colonne en UTF-16 aussi : deux astraux (2 unités chacun) puis « b »
        // le placent en colonne 5.
        let m = find_matches("😀😀b", "b", OPTS);
        assert_eq!((m[0].line, m[0].col), (1, 5));
    }

    #[test]
    fn case_folding_that_grows_does_not_corrupt_lengths() {
        // 'İ' (BMP, 1 unité UTF-16) se replie en « i̇ » (DEUX chars) : la
        // longueur remplacée doit rester celle du TEXTE, pas de la forme
        // repliée — 1 ici, et l'occurrence suivante n'est pas décalée.
        let ci = SearchOptions {
            case_sensitive: false,
            whole_word: false,
        };
        let m = find_matches("İx", "İ", ci);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].len, 1);
        let m = find_matches("İx", "x", ci);
        assert_eq!((m[0].index, m[0].col), (1, 2));
    }

    #[test]
    fn replace_all_swaps_every_occurrence() {
        let ci = SearchOptions {
            case_sensitive: false,
            whole_word: false,
        };
        assert_eq!(replace_all("un deux un", "un", "1", OPTS), "1 deux 1");
        assert_eq!(replace_all("Un un UN", "un", "1", ci), "1 1 1");
        assert_eq!(replace_all("rien", "x", "1", OPTS), "rien");
    }

    #[test]
    fn replace_all_with_multichar_replacement() {
        assert_eq!(replace_all("a b a", "a", "xx", OPTS), "xx b xx");
    }
}
