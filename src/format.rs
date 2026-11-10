//! Le format d'affichage d'un compteur, par ex. `{value}[ / {goal}]` → « 3 / 10 ».
//!
//! Règles (voir F1 dans les specs) :
//! - `{nom}` est remplacé par une variable connue, en anglais ou en français :
//!   `{value}`/`{valeur}`, `{goal}`/`{objectif}`, `{remaining}`/`{reste}`, `{ratio}`
//!   (et `{ratio:p}` pour un pourcentage). Les noms ne tiennent pas compte des majuscules.
//! - Une variable inconnue reste affichée telle quelle (`{valeu}`) : l'erreur se voit.
//! - `[ ... ]` est une partie CONDITIONNELLE, affichée seulement si l'objectif est > 0,
//!   à condition de contenir une variable liée à l'objectif (`goal`, `remaining`, `ratio`).
//!   Sinon, ce sont de simples crochets affichés : `[Coffres] {value}` → « [Coffres] 3 ».
//! - Un `{` ou un `[` sans fermeture est affiché tel quel.
//! - Doubler un caractère l'affiche littéralement : `{{` → `{`, `[[` → `[` (rarement utile).

/// Langue utilisée pour formater les nombres (virgule décimale, espace avant « % »).
/// La langue sera détectée et réglable à l'étape 9.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    French,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "choix de la langue à l'étape 9")
    )]
    English,
}

/// Les valeurs disponibles pour le format.
#[derive(Debug, Clone, Copy)]
pub struct Values {
    pub value: u64,
    /// 0 = pas d'objectif.
    pub goal: u64,
}

/// Les variables connues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variable {
    Value,
    Goal,
    Remaining,
    Ratio { percent: bool },
}

impl Variable {
    /// Reconnaît le contenu d'accolades, par ex. `value`, `Objectif`, `ratio:p`.
    /// `None` si ce n'est pas une variable connue.
    fn parse(content: &str) -> Option<Self> {
        // `split_once(':')` coupe en deux au premier `:` ; `None` s'il n'y en a pas.
        let (name, spec) = match content.split_once(':') {
            Some((name, spec)) => (name, Some(spec)),
            None => (content, None),
        };
        // `to_lowercase()` : insensible aux majuscules. `.trim()` : ignore les espaces autour.
        let name = name.trim().to_lowercase();
        // `as_str()` : on compare le `String` avec des textes fixes (`&str`).
        // `(a, b)` : un TUPLE, pour faire un `match` sur deux valeurs à la fois.
        match (name.as_str(), spec) {
            ("value" | "valeur", None) => Some(Variable::Value),
            ("goal" | "objectif", None) => Some(Variable::Goal),
            ("remaining" | "reste", None) => Some(Variable::Remaining),
            ("ratio", None) => Some(Variable::Ratio { percent: false }),
            ("ratio", Some("p")) => Some(Variable::Ratio { percent: true }),
            _ => None,
        }
    }

    /// Cette variable dépend-elle de l'objectif ?
    fn depends_on_goal(self) -> bool {
        !matches!(self, Variable::Value)
    }

    fn render(self, values: Values, language: Language) -> String {
        match self {
            Variable::Value => values.value.to_string(),
            Variable::Goal => values.goal.to_string(),
            // Objectif dépassé → 0, jamais négatif.
            Variable::Remaining => values.goal.saturating_sub(values.value).to_string(),
            Variable::Ratio { percent } => {
                // Pas d'objectif → ratio 0 (évite une division par zéro).
                let ratio = if values.goal == 0 {
                    0.0
                } else {
                    values.value as f64 / values.goal as f64
                };
                if percent {
                    format_percent(ratio, language)
                } else {
                    format_decimal(ratio, language)
                }
            }
        }
    }
}

/// Un morceau de format, une fois analysé.
#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Text(String),
    Variable(Variable),
    /// Partie affichée seulement si l'objectif est > 0.
    Conditional(Vec<Segment>),
}

/// Formate un compteur selon un modèle. C'est la seule fonction publique du module.
pub fn render(template: &str, values: Values, language: Language) -> String {
    render_segments(&parse(template), values, language)
}

fn render_segments(segments: &[Segment], values: Values, language: Language) -> String {
    let mut out = String::new();
    for segment in segments {
        match segment {
            Segment::Text(text) => out.push_str(text),
            Segment::Variable(variable) => out.push_str(&variable.render(values, language)),
            Segment::Conditional(inner) => {
                if values.goal > 0 {
                    out.push_str(&render_segments(inner, values, language));
                }
            }
        }
    }
    out
}

/// Analyse un modèle de format en une liste de segments.
fn parse(template: &str) -> Vec<Segment> {
    // On travaille sur un tableau de caractères pour pouvoir regarder « le suivant ».
    // (En Rust, une chaîne est en UTF-8 : on ne peut pas l'indexer directement par caractère.)
    let chars: Vec<char> = template.chars().collect();
    let mut position = 0;
    let (segments, _closed) = parse_sequence(&chars, &mut position, false);
    segments
}

/// Analyse à partir de `position` jusqu'à la fin, ou jusqu'au `]` fermant si `in_conditional`.
/// Renvoie les segments et `true` si un `]` fermant a été trouvé.
///
/// `position: &mut usize` : la position est PARTAGÉE avec l'appelant, qui voit où on s'est arrêté.
fn parse_sequence(
    chars: &[char],
    position: &mut usize,
    in_conditional: bool,
) -> (Vec<Segment>, bool) {
    let mut segments = Vec::new();

    while *position < chars.len() {
        let current = chars[*position];
        // `.get(i)` renvoie une `Option` : `None` si on est au bout (pas de plantage).
        let next = chars.get(*position + 1).copied();

        match current {
            // Caractères doublés → caractère littéral.
            '{' | '}' | '[' | ']' if next == Some(current) => {
                push_text(&mut segments, current);
                *position += 2;
            }
            // Fin de la partie conditionnelle.
            ']' if in_conditional => {
                *position += 1;
                return (segments, true);
            }
            '{' => {
                *position += 1;
                match find_closing_brace(chars, *position) {
                    Some(close) => {
                        let content: String = chars[*position..close].iter().collect();
                        match Variable::parse(&content) {
                            Some(variable) => segments.push(Segment::Variable(variable)),
                            // Variable inconnue : on garde le texte d'origine, accolades comprises.
                            None => {
                                for c in format!("{{{content}}}").chars() {
                                    push_text(&mut segments, c);
                                }
                            }
                        }
                        *position = close + 1;
                    }
                    None => push_text(&mut segments, '{'), // pas de fermeture : littéral
                }
            }
            // Début d'une partie conditionnelle (pas d'imbrication : un `[` à l'intérieur est littéral).
            '[' if !in_conditional => {
                *position += 1;
                // Appel RÉCURSIF : la fonction s'appelle elle-même pour l'intérieur des crochets.
                let (inner, closed) = parse_sequence(chars, position, true);
                if closed && inner.iter().any(segment_depends_on_goal) {
                    segments.push(Segment::Conditional(inner));
                } else {
                    // Pas une condition : de simples crochets, affichés.
                    push_text(&mut segments, '[');
                    for segment in inner {
                        push_segment(&mut segments, segment);
                    }
                    if closed {
                        push_text(&mut segments, ']');
                    }
                }
            }
            other => {
                push_text(&mut segments, other);
                *position += 1;
            }
        }
    }

    (segments, false)
}

/// Cherche le `}` qui ferme une variable. S'arrête sur un `{` (ex. « {a {value} » : le premier `{` est littéral).
fn find_closing_brace(chars: &[char], start: usize) -> Option<usize> {
    for (offset, c) in chars[start..].iter().enumerate() {
        match c {
            '}' => return Some(start + offset),
            '{' => return None,
            _ => {}
        }
    }
    None
}

fn segment_depends_on_goal(segment: &Segment) -> bool {
    matches!(segment, Segment::Variable(v) if v.depends_on_goal())
}

/// Ajoute un caractère au dernier segment de texte (ou en crée un).
fn push_text(segments: &mut Vec<Segment>, c: char) {
    // `last_mut()` : référence modifiable vers le dernier élément, s'il existe.
    if let Some(Segment::Text(text)) = segments.last_mut() {
        text.push(c);
    } else {
        segments.push(Segment::Text(c.to_string()));
    }
}

/// Ajoute un segment en fusionnant les textes consécutifs.
fn push_segment(segments: &mut Vec<Segment>, segment: Segment) {
    match segment {
        Segment::Text(text) => text.chars().for_each(|c| push_text(segments, c)),
        other => segments.push(other),
    }
}

/// 0.3 → « 30 % » (français) ou « 30% » (anglais), sans décimale.
fn format_percent(ratio: f64, language: Language) -> String {
    let percent = (ratio * 100.0).round();
    match language {
        Language::French => format!("{percent} %"),
        Language::English => format!("{percent}%"),
    }
}

/// 0.3 → « 0,3 » (français) ou « 0.3 » (anglais), au plus 2 décimales, sans zéros inutiles.
fn format_decimal(ratio: f64, language: Language) -> String {
    // `{:.2}` : 2 décimales. Puis on retire les zéros finaux, et le point s'il reste seul.
    let text = format!("{ratio:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match language {
        Language::French => text.replace('.', ","),
        Language::English => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Raccourci pour les tests : format en français.
    fn fr(template: &str, value: u64, goal: u64) -> String {
        render(template, Values { value, goal }, Language::French)
    }

    fn en(template: &str, value: u64, goal: u64) -> String {
        render(template, Values { value, goal }, Language::English)
    }

    #[test]
    fn plain_value() {
        assert_eq!(fr("{value}", 3, 0), "3");
    }

    #[test]
    fn default_format_with_and_without_goal() {
        assert_eq!(fr("{value}[ / {goal}]", 3, 10), "3 / 10");
        assert_eq!(fr("{value}[ / {goal}]", 3, 0), "3");
    }

    #[test]
    fn french_names_work_too() {
        assert_eq!(
            fr("{valeur} / {objectif} (reste {reste})", 3, 10),
            "3 / 10 (reste 7)"
        );
    }

    #[test]
    fn names_ignore_case() {
        assert_eq!(fr("{Value} {GOAL}", 3, 10), "3 10");
    }

    #[test]
    fn remaining_never_negative() {
        assert_eq!(fr("{remaining}", 12, 10), "0");
    }

    #[test]
    fn ratio_as_percent() {
        assert_eq!(fr("{ratio:p}", 3, 10), "30 %");
        assert_eq!(en("{ratio:p}", 3, 10), "30%");
        assert_eq!(fr("{ratio:p}", 1, 3), "33 %"); // arrondi
    }

    #[test]
    fn ratio_as_decimal() {
        assert_eq!(fr("{ratio}", 3, 10), "0,3");
        assert_eq!(en("{ratio}", 3, 10), "0.3");
        assert_eq!(fr("{ratio}", 10, 10), "1");
        assert_eq!(fr("{ratio}", 1, 3), "0,33");
    }

    #[test]
    fn ratio_without_goal_is_zero() {
        assert_eq!(fr("{ratio:p}", 3, 0), "0 %");
    }

    #[test]
    fn unknown_variable_is_shown_as_is() {
        assert_eq!(fr("{valeu} coffres", 3, 0), "{valeu} coffres");
        assert_eq!(fr("{value:p}", 3, 10), "{value:p}"); // `:p` n'a de sens que pour ratio
    }

    #[test]
    fn brackets_without_goal_variable_are_plain_text() {
        assert_eq!(fr("[Coffres] {value}", 3, 0), "[Coffres] 3");
        assert_eq!(fr("[{value}]", 3, 0), "[3]");
    }

    #[test]
    fn unclosed_brace_or_bracket_is_plain_text() {
        assert_eq!(fr("{value} {", 3, 0), "3 {");
        assert_eq!(fr("{value} [ / {goal}", 3, 10), "3 [ / 10");
        assert_eq!(fr("{a {value}", 3, 0), "{a 3");
    }

    #[test]
    fn doubled_characters_are_literal() {
        assert_eq!(fr("{{value}}", 3, 0), "{value}");
        assert_eq!(fr("[[{goal}]]", 3, 10), "[10]");
    }

    #[test]
    fn conditional_with_remaining_and_percent() {
        let template = "{value}[ ({ratio:p}, reste {remaining})]";
        assert_eq!(fr(template, 3, 10), "3 (30 %, reste 7)");
        assert_eq!(fr(template, 3, 0), "3");
    }

    #[test]
    fn empty_template_gives_empty_text() {
        assert_eq!(fr("", 3, 10), "");
    }
}
