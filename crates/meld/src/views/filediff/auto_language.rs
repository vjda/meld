//! Content-based language detection for panes without a file (S1–S4, S8–S9).
//!
//! A pane with no path has no file name to guess a syntax from, so the text
//! itself is the only evidence. [`betlang`] ranks 48 source languages with a
//! small embedded model; Meld trusts its top choice only when it is confident
//! (a score of at least [`MIN_SCORE`]), keeps the current highlighting when it
//! is not (S4: no evidence is plain text, and an in-progress document must not
//! flicker between guesses) and stops highlighting when the pane is empty.
//!
//! Only languages in [`LANGUAGES`] switch the highlighting, so a label Meld
//! cannot highlight leaves the pane as it is: S3 asks for YAML, JSON and
//! Markdown, S11 covers the rest of the detector's labels wherever Meld has a
//! highlighting mode for the same language, and what stays unmapped is a
//! documented gap either in Meld's language table or in this build's syntax
//! set (unnamed labels and prose stay plain text).
//!
//! Model and licence: `betlang` 0.1.1 is MIT licensed and embeds a
//! distilled model trained from Google's [Magika] (Apache-2.0). Detection
//! runs locally: no network access, and the model is compiled into the
//! binary.
//!
//! [Magika]: https://github.com/google/magika

use meld_widgets::Highlighter;

/// How much of the pane's text the detector reads (bytes). The model
/// window is fixed, so a bounded prefix keeps every detection under a
/// millisecond even for a huge buffer.
pub(super) const SAMPLE_BYTES: usize = 4096;

/// Confidence at or above which a detection is applied (S9).
const MIN_SCORE: f32 = 0.5;

/// Detector label slugs mapped to a Meld language ID (S3, S9, S11).
///
/// A label is listed only when its Meld ID resolves to a highlighting mode
/// in this build, so detection never selects a mode that highlights
/// nothing; labels Meld cannot highlight stay out and keep the pane as it
/// is. The tests in this module list the labels that stay out, and why.
const LANGUAGES: &[(&str, &str)] = &[
    ("json", "json"),
    ("yaml", "yaml"),
    ("markdown", "markdown"),
    ("xml", "xml"),
    ("html", "html"),
    ("groovy", "groovy"),
    ("python", "python3"),
    ("javascript", "js"),
    ("typescript", "typescript"),
    ("java", "java"),
    ("go", "go"),
    ("rust", "rust"),
    // S11: the rest of the detector's labels whose Meld mode exists here.
    ("batch", "dosbatch"),
    ("c", "c"),
    ("cpp", "cpp"),
    ("cs", "c-sharp"),
    ("css", "css"),
    ("erlang", "erlang"),
    ("haskell", "haskell"),
    ("lisp", "commonlisp"),
    ("lua", "lua"),
    ("objectivec", "objc"),
    ("ocaml", "ocaml"),
    ("perl", "perl"),
    ("php", "php"),
    ("r", "r"),
    ("ruby", "ruby"),
    ("scala", "scala"),
    ("shell", "sh"),
    ("sql", "sql"),
];

/// What the pane's text looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Detected {
    /// Confident detection of a language Meld can highlight.
    Language(&'static str),
    /// The pane is empty: stop highlighting (S4).
    Plain,
    /// Not enough evidence: keep the current highlighting (S4, S9).
    Unknown,
}

/// Detects the language of `text` (S1, S3, S8).
///
/// `betlang` ranks the languages of a bounded prefix; its top choice is
/// applied only when it is confident and Meld has a highlighting mode
/// for it, so unknown content keeps whatever the pane shows.
pub(super) fn detect(text: &str) -> Detected {
    let text = prefix(text, SAMPLE_BYTES);
    if text.trim().is_empty() {
        return Detected::Plain;
    }
    let detection = betlang::detect(text);
    // `top_languages` yields `(probability, language)`, most likely first.
    let Some((score, language)) = detection.top_languages().next() else {
        // The input was too short to fill the model window.
        return Detected::Unknown;
    };
    if score < MIN_SCORE {
        return Detected::Unknown;
    }
    match meld_language(language.slug()) {
        Some(id) if Highlighter::for_language(id).syntax_name().is_some() => Detected::Language(id),
        _ => Detected::Unknown,
    }
}

/// Detects the language of a pane's `lines`, as [`detect`] does for text
/// already reduced to a sample.
///
/// The detector reads a bounded [`sample`], so the sample alone cannot
/// tell an empty pane from one whose first [`SAMPLE_BYTES`] are blank:
/// emptiness is judged on the whole buffer, and blank sample text with
/// content further down is no evidence (`Unknown`, keep the current
/// language) instead of clearing it (S4, S9).
pub(super) fn detect_lines(lines: &[String]) -> Detected {
    let detected = detect(&sample(lines));
    if detected == Detected::Plain && lines.iter().any(|line| !line.trim().is_empty()) {
        return Detected::Unknown;
    }
    detected
}

/// A bounded prefix of a pane's `lines`, for [`detect`].
///
/// Whole lines are kept up to [`SAMPLE_BYTES`], cutting a long line on a
/// character boundary, so the caller never copies a huge buffer. A cut
/// line ends the sample: joining a later line onto it would splice a
/// fragment of that line into a truncated one.
pub(super) fn sample(lines: &[String]) -> String {
    let mut text = String::with_capacity(SAMPLE_BYTES);
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            if text.len() >= SAMPLE_BYTES {
                break;
            }
            text.push('\n');
        }
        let room = SAMPLE_BYTES.saturating_sub(text.len());
        if room == 0 {
            break;
        }
        let kept = prefix(line, room);
        text.push_str(kept);
        if kept.len() < line.len() {
            break;
        }
    }
    text
}

/// The longest prefix of `text` that fits in `bytes` without splitting a
/// character.
fn prefix(text: &str, bytes: usize) -> &str {
    if text.len() <= bytes {
        return text;
    }
    let mut end = bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// The Meld language ID for a detector label slug, when Meld can
/// highlight it.
fn meld_language(slug: &str) -> Option<&'static str> {
    let (_, id) = LANGUAGES.iter().find(|(label, _)| *label == slug)?;
    Some(id)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use meld_widgets::Action;
    use meld_widgets::EditAction;

    use super::super::find::FindMessage;
    use super::super::{FileDiff, Pane};
    use super::*;

    /// A JSON object; the model scores it comfortably above the gate.
    const JSON: &str = r#"{
  "name": "meld",
  "version": "4.0.0-alpha.1",
  "keywords": ["diff", "merge", "syntax"],
  "private": false,
  "author": { "name": "Meld developers", "email": "meld@example.org" }
}"#;

    /// A JSON array: the other common top-level shape.
    const JSON_ARRAY: &str = r#"[
  { "id": 1, "name": "alpha", "enabled": true },
  { "id": 2, "name": "beta", "enabled": false },
  { "id": 3, "name": "gamma", "enabled": true }
]"#;

    /// JSON that stopped mid-word: below the gate, the pane keeps what it
    /// shows instead of jumping to another language.
    const PARTIAL_JSON: &str = "{\"a\": 1, \"b";

    /// Nested YAML, as in a manifest.
    const YAML: &str =
        "service:\n  name: meld\n  replicas: 2\n  ports:\n    - port: 8080\n      name: http\n";

    /// A Markdown document.
    const MARKDOWN: &str = "# Unnamed pane syntax autodetection\n\nMeld highlights the text of a pane that has no file behind it.\n\n## Behaviour\n\n- confident detections switch the highlighting\n- everything else stays plain text\n\nSee `auto_language.rs`.\n";

    /// Prose with colons: the classic YAML false positive, below the gate.
    const PROSE: &str = "Remember the plan: first we detect, then we highlight. This is prose with colons: like this: and that.\n";

    /// Log output, also below the gate.
    const LOGS: &str =
        "2026-02-14 10:00:01 INFO  starting comparison\n2026-02-14 10:00:02 WARN  slow highlight\n";

    /// An env file: the model's best label is not one Meld highlights
    /// confidently, so the pane keeps its language.
    const ENV: &str = "PATH=/usr/bin:/bin\nHOME=/home/user\nRUST_LOG=info\nMELD_MODE=compare\n";

    /// TOML, which the detector recognises but this build cannot highlight.
    const TOML: &str =
        "[package]\nname = \"meld\"\nversion = \"4.0.0\"\n\n[dependencies]\nbetlang = \"0.1.1\"\n";

    /// A Dockerfile: same situation as [`TOML`].
    const DOCKERFILE: &str =
        "FROM rust:1.88 AS build\nWORKDIR /src\nCOPY . .\nRUN cargo build --release\n";

    /// A CSV table, which has no detector label and no Meld mode.
    const CSV: &str = "id,name,enabled\n1,alpha,true\n2,beta,false\n3,gamma,true\n";

    // One canonical short file per language S11 added to the mapping (see
    // `SMOKE`): a real file, not a fragment, so the smoke rows measure the
    // same gate the editor uses. Objective-C and PHP need a whole class:
    // their fragments score below the gate.
    const C: &str = "#include <stdio.h>\n\nint main(void) {\n    printf(\"hello, world\\n\");\n    return 0;\n}\n";
    const CPP: &str = "#include <iostream>\n\nint main() {\n    std::cout << \"hello\" << std::endl;\n    return 0;\n}\n";
    const CS: &str = "using System;\n\nnamespace Demo {\n    public class Program {\n        static void Main() {\n            Console.WriteLine(\"hi from .NET\");\n        }\n    }\n}\n";
    const CSS: &str = "body {\n  margin: 0;\n  font-family: sans-serif;\n}\n\n.btn { color: #333; padding: 4px 8px; }\n";
    const SHELL: &str =
        "#!/bin/sh\nset -eu\n\nfor f in *.txt; do\n    echo \"processing $f\"\ndone\n";
    const SQL: &str = "CREATE TABLE users (\n    id INTEGER PRIMARY KEY,\n    name TEXT NOT NULL\n);\n\nSELECT id, name FROM users WHERE active = 1 ORDER BY name;\n";
    const RUBY: &str = "class Greeter\n  def initialize(name)\n    @name = name\n  end\n\n  def greet\n    puts \"Hello, #{@name}!\"\n  end\nend\n\nGreeter.new(\"world\").greet\n";
    const PHP: &str = "<?php\n\nclass Greeter\n{\n    public function __construct(private string $name)\n    {\n    }\n\n    public function greet(): string\n    {\n        return 'Hello, ' . $this->name . '!';\n    }\n}\n\necho (new Greeter('world'))->greet();\n";
    const PERL: &str = "use strict;\nuse warnings;\n\nmy @files = glob(\"*.txt\");\nforeach my $f (@files) {\n    print \"found $f\\n\";\n}\n";
    const LUA: &str =
        "local function greet(name)\n    print(\"Hello, \" .. name)\nend\n\ngreet(\"world\")\n";
    const HASKELL: &str = "module Main where\n\nmain :: IO ()\nmain = do\n    let xs = [1 .. 10]\n    print (sum xs)\n";
    const OCAML: &str = "let rec fact n =\n  if n <= 1 then 1\n  else n * fact (n - 1)\n\nlet () = print_int (fact 5)\n";
    const OBJC: &str = "#import <Foundation/Foundation.h>\n\n@interface Greeter : NSObject\n\n@property (nonatomic, copy) NSString *name;\n\n- (instancetype)initWithName:(NSString *)name;\n- (NSString *)greet;\n\n@end\n\n@implementation Greeter\n\n- (instancetype)initWithName:(NSString *)name\n{\n    self = [super init];\n    if (self) {\n        _name = [name copy];\n    }\n    return self;\n}\n\n- (NSString *)greet\n{\n    return [NSString stringWithFormat:@\"Hello, %@!\", self.name];\n}\n\n@end\n";
    const ERLANG: &str = "-module(greet).\n-export([hello/1]).\n\nhello(Name) ->\n    io:format(\"Hello, ~s!~n\", [Name]).\n";
    const BATCH: &str =
        "@echo off\nsetlocal\n\nfor %%f in (*.txt) do (\n    echo Processing %%f\n)\n\nendlocal\n";
    const R: &str = "data <- read.csv(\"values.csv\")\nsummary(data)\nmean(data$value, na.rm = TRUE)\nplot(data$value, type = \"l\")\n";
    const SCALA: &str = "object Main extends App {\n  val values = List(1, 2, 3)\n  println(values.map(_ * 2))\n}\n";
    const LISP: &str = "(defun factorial (n)\n  (if (<= n 1)\n      1\n      (* n (factorial (- n 1)))))\n\n(format t \"~a~%\" (factorial 5))\n";

    /// What `Message::Paste` does: the pane action applies the edit and
    /// runs the same post-edit detection as typing does.
    fn paste(diff: &mut FileDiff, pane: usize, text: &str) {
        let _ = diff.pane_action(
            pane,
            Action::Edit(EditAction::Paste(Arc::new(text.to_string()))),
        );
    }

    /// Replaces a pane's whole text, as selecting everything and pasting.
    fn replace(diff: &mut FileDiff, pane: usize, text: &str) {
        let _ = diff.pane_action(pane, Action::SelectAll);
        paste(diff, pane, text);
    }

    /// Replays a comparison-driven change on a pane's buffer, as merging
    /// and chunk actions do.
    fn replay(diff: &mut FileDiff, pane: usize, text: &str) {
        let Some(cmp) = diff.cmp.as_mut() else {
            panic!("the test comparison is missing");
        };
        match cmp.set_text(pane, text) {
            Ok(outcome) => diff.apply_outcome(outcome),
            Err(err) => panic!("set_text failed: {err}"),
        }
    }

    fn language(diff: &FileDiff, pane: usize) -> Option<&'static str> {
        diff.panes[pane].language
    }

    fn syntax(diff: &FileDiff, pane: usize) -> Option<&'static str> {
        diff.panes[pane].highlighter.syntax_name()
    }

    /// Every label `betlang` 0.1.1 can return ([`betlang::Language::slug`]),
    /// so the mapped and the excluded tables below have to account for all
    /// of them (S11).
    const BETLANG_LABELS: &[&str] = &[
        "asm",
        "batch",
        "c",
        "clojure",
        "cmake",
        "cobol",
        "cpp",
        "cs",
        "css",
        "dart",
        "dockerfile",
        "elixir",
        "erlang",
        "gemfile",
        "gemspec",
        "go",
        "gradle",
        "groovy",
        "haskell",
        "html",
        "ini",
        "java",
        "javascript",
        "json",
        "julia",
        "kotlin",
        "lisp",
        "lua",
        "markdown",
        "objectivec",
        "ocaml",
        "perl",
        "php",
        "powershell",
        "python",
        "r",
        "ruby",
        "rust",
        "scala",
        "shell",
        "sql",
        "swift",
        "toml",
        "typescript",
        "vba",
        "verilog",
        "xml",
        "yaml",
    ];

    /// Labels S11 leaves unmapped, as `(label, Meld id, why)`: either Meld
    /// knows the language but this build has no syntax for it, or Meld has no
    /// language of its own for the label. Mapping one of those would select a
    /// highlighting mode that highlights nothing, and reusing another
    /// language's mode would highlight a different grammar.
    const EXCLUDED: &[(&str, Option<&str>, &str)] = &[
        ("asm", None, "Meld has no assembly language"),
        ("clojure", None, "Meld's language table has no Clojure"),
        ("cmake", Some("cmake"), "no CMake syntax in this build"),
        ("cobol", None, "Meld has no COBOL language"),
        ("dart", Some("dart"), "no Dart syntax in this build"),
        (
            "dockerfile",
            Some("docker"),
            "no Dockerfile syntax in this build",
        ),
        ("elixir", Some("elixir"), "no Elixir syntax in this build"),
        ("gemfile", None, "a Gemfile has no Meld language of its own"),
        ("gemspec", None, "a gemspec has no Meld language of its own"),
        ("gradle", None, "a Gradle build script is not Groovy source"),
        ("ini", Some("ini"), "no .ini syntax in this build"),
        ("julia", Some("julia"), "no Julia syntax in this build"),
        ("kotlin", Some("kotlin"), "no Kotlin syntax in this build"),
        (
            "powershell",
            Some("powershell"),
            "no PowerShell syntax in this build",
        ),
        ("swift", Some("swift"), "no Swift syntax in this build"),
        ("toml", Some("toml"), "no TOML syntax in this build"),
        ("vba", Some("vbnet"), "VB.NET is a different dialect"),
        (
            "verilog",
            Some("verilog"),
            "no Verilog syntax in this build",
        ),
    ];

    #[test]
    fn every_mapped_language_has_a_highlighting_mode() {
        for (slug, id) in LANGUAGES {
            assert_eq!(
                meld_files::languages::language(id).map(|language| language.id),
                Some(*id),
                "{slug} maps to {id}, which is not a Meld language"
            );
            assert!(
                Highlighter::for_language(id).syntax_name().is_some(),
                "{slug} maps to {id}, which has no highlighting"
            );
            assert_eq!(meld_language(slug), Some(*id));
        }
        // A label nobody knows is not mapped either.
        assert_eq!(meld_language("not-a-language"), None);
    }

    #[test]
    fn every_betlang_label_is_mapped_or_excluded() {
        for label in BETLANG_LABELS {
            let mapped = meld_language(label).is_some();
            let excluded = EXCLUDED.iter().any(|(excluded, _, _)| excluded == label);
            assert!(
                mapped != excluded,
                "{label}: mapped={mapped} excluded={excluded}"
            );
        }
        assert_eq!(
            LANGUAGES.len() + EXCLUDED.len(),
            BETLANG_LABELS.len(),
            "every label is mapped or excluded exactly once"
        );
    }

    #[test]
    fn excluded_labels_record_why_they_cannot_be_mapped() {
        for &(label, id, reason) in EXCLUDED {
            assert!(!reason.is_empty(), "{label} is excluded without a reason");
            assert_eq!(meld_language(label), None, "{label} is excluded but mapped");
            match id {
                // Meld knows the language: the exclusion is this build's
                // missing syntax, and a future syntax set must flip this.
                Some(id) => {
                    assert_eq!(
                        meld_files::languages::language(id).map(|language| language.id),
                        Some(id),
                        "{label} excludes {id}, which is not a Meld language"
                    );
                    assert!(
                        Highlighter::for_language(id).syntax_name().is_none(),
                        "{id} now has a highlighting mode; revisit the label excluded for it"
                    );
                }
                // Meld has no language of its own for the label.
                None => assert!(
                    meld_files::languages::language(label).is_none(),
                    "Meld gained a {label} language; map the label in LANGUAGES"
                ),
            }
        }
    }

    #[test]
    fn the_required_languages_are_mapped() {
        for slug in ["json", "yaml", "markdown"] {
            assert_eq!(meld_language(slug), Some(slug));
        }
    }

    #[test]
    fn confident_json_is_detected() {
        assert_eq!(detect(JSON), Detected::Language("json"));
        assert_eq!(detect(JSON_ARRAY), Detected::Language("json"));
    }

    #[test]
    fn confident_yaml_and_markdown_are_detected() {
        assert_eq!(detect(YAML), Detected::Language("yaml"));
        assert_eq!(detect(MARKDOWN), Detected::Language("markdown"));
    }

    #[test]
    fn incomplete_json_stays_unknown_instead_of_guessing() {
        assert_eq!(detect(PARTIAL_JSON), Detected::Unknown);
        assert_eq!(detect("{\"a\": 1,"), Detected::Unknown);
        assert_eq!(detect("{"), Detected::Unknown);
    }

    #[test]
    fn prose_and_logs_stay_unknown() {
        assert_eq!(detect(PROSE), Detected::Unknown);
        assert_eq!(detect(LOGS), Detected::Unknown);
    }

    #[test]
    fn empty_content_clears_the_highlighting() {
        assert_eq!(detect(""), Detected::Plain);
        assert_eq!(detect(" \n\t\n"), Detected::Plain);
    }

    #[test]
    fn labels_meld_cannot_highlight_stay_unknown() {
        assert_eq!(detect(ENV), Detected::Unknown);
    }

    #[test]
    fn content_of_unmappable_labels_keeps_the_pane_as_it_is() {
        // The detector is confident about these, but the labels have no
        // highlighting mode here (S9, S11), so the pane stays as it is
        // instead of selecting a mode that highlights nothing.
        assert_eq!(detect(TOML), Detected::Unknown);
        assert_eq!(detect(DOCKERFILE), Detected::Unknown);
        // CSV has neither a detector label nor a Meld mode.
        assert_eq!(detect(CSV), Detected::Unknown);
    }

    #[test]
    fn only_a_bounded_prefix_of_a_long_text_is_read() {
        // The first lines of a long JSON document are read, so a huge
        // buffer costs the same as a small one and still detects.
        let mut long = String::from(JSON);
        for _ in 0..2000 {
            long.push_str("\n// padding");
        }
        assert!(long.len() > 4 * SAMPLE_BYTES);
        assert_eq!(detect(&long), Detected::Language("json"));
    }

    #[test]
    fn the_sample_keeps_whole_characters_and_stays_bounded() {
        let wide = "é".repeat(4 * SAMPLE_BYTES);
        let bounded = sample(&[wide.clone(), "name: meld".to_string()]);
        assert!(bounded.len() <= SAMPLE_BYTES + 1, "{} bytes", bounded.len());
        assert!(bounded.is_char_boundary(bounded.len()));
        assert!(bounded.starts_with('é'));
        // The second line never fits, so nothing of it is in the sample.
        assert!(!bounded.contains("name:"));

        let short = sample(&["name: meld".to_string(), "replicas: 2".to_string()]);
        assert_eq!(short, "name: meld\nreplicas: 2");
    }

    #[test]
    fn a_line_cut_on_a_character_boundary_ends_the_sample() {
        // A line whose last character does not fit leaves a few spare
        // bytes; they must not be filled with the next line's beginning.
        let cut = format!("{}{}", "a".repeat(SAMPLE_BYTES - 3), "🦀");
        let bounded = sample(&[cut, "name: meld".to_string()]);
        assert!(bounded.len() <= SAMPLE_BYTES, "{} bytes", bounded.len());
        assert!(bounded.is_char_boundary(bounded.len()));
        assert_eq!(bounded, "a".repeat(SAMPLE_BYTES - 3));
    }

    #[test]
    fn huge_multibyte_buffers_are_bounded_and_panic_free() {
        let lines = vec!["héllo 🦀 wörld".to_string(); 20_000];
        let bounded = sample(&lines);
        assert!(bounded.len() <= SAMPLE_BYTES + 1, "{} bytes", bounded.len());
        assert!(bounded.is_char_boundary(bounded.len()));
        // Detection reads the bounded prefix and neither panics nor
        // treats the visible text as an empty pane.
        assert_ne!(detect(&bounded), Detected::Plain);
    }

    #[test]
    fn pasting_json_switches_an_unnamed_pane_to_json() {
        let mut diff = FileDiff::for_test(2);
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());

        paste(&mut diff, 0, JSON);

        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(syntax(&diff, 0), Some("JSON"));
        assert_eq!(diff.panes[0].buffer.lines().join("\n"), JSON);
    }

    #[test]
    fn pasting_yaml_and_markdown_switches_to_them() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, YAML);
        assert_eq!(language(&diff, 0), Some("yaml"));
        assert_eq!(syntax(&diff, 0), Some("YAML"));

        paste(&mut diff, 1, MARKDOWN);
        assert_eq!(language(&diff, 1), Some("markdown"));
        assert_eq!(syntax(&diff, 1), Some("Markdown"));
    }

    #[test]
    fn inconclusive_text_leaves_an_empty_pane_plain() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, PROSE);
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());

        replace(&mut diff, 0, PARTIAL_JSON);
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());
    }

    #[test]
    fn inconclusive_edits_keep_the_current_language() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, JSON);
        assert_eq!(language(&diff, 0), Some("json"));

        // Typing on: the document is no longer a complete JSON value.
        replace(&mut diff, 0, PARTIAL_JSON);
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(syntax(&diff, 0), Some("JSON"));

        // Something that looks like nothing in particular keeps it too.
        replace(&mut diff, 0, LOGS);
        assert_eq!(language(&diff, 0), Some("json"));
    }

    #[test]
    fn typing_json_character_by_character_detects_it() {
        let mut diff = FileDiff::for_test(2);
        for c in JSON.chars() {
            let _ = diff.pane_action(0, Action::Edit(EditAction::Insert(c)));
        }
        assert_eq!(diff.panes[0].buffer.lines().join("\n"), JSON);
        assert_eq!(language(&diff, 0), Some("json"));
    }

    #[test]
    fn clearing_the_pane_goes_back_to_plain_text() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, JSON);
        assert_eq!(language(&diff, 0), Some("json"));

        replace(&mut diff, 0, "");
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());
        assert_eq!(diff.panes[0].buffer.lines().join("\n"), "");
    }

    #[test]
    fn content_beyond_a_whitespace_sample_keeps_the_language() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, JSON);
        assert_eq!(language(&diff, 0), Some("json"));

        // More blank lines than the detector reads, then the content:
        // the sample is blank, but the pane is not empty (S9).
        let pushed_down = format!("{}\n{JSON}", "\n".repeat(SAMPLE_BYTES + 1));
        replace(&mut diff, 0, &pushed_down);
        assert!(sample(diff.panes[0].buffer.lines()).trim().is_empty());
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(syntax(&diff, 0), Some("JSON"));

        // A pane that really is blank still goes back to plain text.
        replace(&mut diff, 0, "\n\n");
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());
    }

    #[test]
    fn undoing_back_to_empty_goes_back_to_plain_text() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, JSON);
        assert_eq!(language(&diff, 0), Some("json"));

        diff.undo_redo(false);

        assert_eq!(diff.panes[0].buffer.lines().join("\n"), "");
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());
    }

    #[test]
    fn replacing_all_lines_re_detects_the_language() {
        let mut diff = FileDiff::for_test(2);
        // The same JSON, each line commented out: not JSON as it stands.
        let commented: Vec<String> = JSON.lines().map(|l| format!("// {l}")).collect();
        let commented = commented.join("\n");
        paste(&mut diff, 0, &commented);
        assert_ne!(syntax(&diff, 0), Some("JSON"));

        // One replacement per line, so the changes are per-line edits.
        diff.find.text = "// ".to_string();
        diff.find.replacement = String::new();
        let _ = diff.find_message(FindMessage::ReplaceAll);

        assert_eq!(diff.panes[0].buffer.lines().join("\n"), JSON);
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(syntax(&diff, 0), Some("JSON"));
    }

    #[test]
    fn replaying_an_outcome_re_detects_the_language() {
        let mut diff = FileDiff::for_test(2);

        replay(&mut diff, 0, JSON);
        assert_eq!(diff.panes[0].buffer.lines().join("\n"), JSON);
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(syntax(&diff, 0), Some("JSON"));

        // And back to empty: the highlighting stops.
        replay(&mut diff, 0, "");
        assert_eq!(diff.panes[0].buffer.lines().join("\n"), "");
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());
    }

    #[test]
    fn a_language_picked_by_hand_disables_detection() {
        let mut diff = FileDiff::for_test(2);
        diff.panes[0].choose_language(Some("python3"));
        assert_eq!(syntax(&diff, 0), Some("Python"));

        paste(&mut diff, 0, JSON);
        assert_eq!(language(&diff, 0), Some("python3"));
        assert_eq!(syntax(&diff, 0), Some("Python"));
        assert!(diff.panes[0].language_chosen);
    }

    #[test]
    fn plain_text_picked_by_hand_also_sticks() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, YAML);
        assert_eq!(language(&diff, 0), Some("yaml"));

        diff.panes[0].choose_language(None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());

        replace(&mut diff, 0, JSON);
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());
    }

    #[test]
    fn auto_detect_turns_detection_back_on_and_detects_now() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, JSON);
        diff.panes[0].choose_language(Some("python3"));
        assert_eq!(language(&diff, 0), Some("python3"));

        diff.panes[0].use_auto_detect();
        assert!(!diff.panes[0].language_chosen);
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(syntax(&diff, 0), Some("JSON"));
    }

    #[test]
    fn auto_detect_without_evidence_falls_back_to_plain_text() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, PROSE);
        diff.panes[0].choose_language(Some("rust"));
        assert_eq!(language(&diff, 0), Some("rust"));

        // The language was picked by hand, so it is not evidence: the
        // conservative answer for prose is plain text (S4).
        diff.panes[0].use_auto_detect();
        assert_eq!(language(&diff, 0), None);
        assert!(diff.panes[0].highlighter.syntax_name().is_none());
    }

    #[test]
    fn auto_detect_is_offered_only_for_unnamed_panes() {
        // The selector offers Auto-detect in unnamed panes and marks it
        // while the pane's language comes from its text (S2).
        let mut diff = FileDiff::for_test(2);
        assert!(diff.panes[0].auto_detects());

        diff.panes[0].choose_language(Some("json"));
        assert!(!diff.panes[0].auto_detects());
        assert!(diff.panes[0].file.path.is_none());

        let mut pane = Pane::new();
        pane.file.path = Some("/tmp/meld-test-file".into());
        assert!(!pane.auto_detects());
    }

    #[test]
    fn a_file_backed_pane_keeps_the_language_of_its_file() {
        let mut diff = FileDiff::for_test(2);
        let guessed = ["python3", "rust"];
        for (pane, id) in diff.panes.iter_mut().zip(guessed) {
            pane.file.path = Some(PathBuf::from("/tmp/meld-test-file"));
            pane.set_language(Some(id));
        }

        paste(&mut diff, 0, JSON);
        paste(&mut diff, 1, JSON);
        assert_eq!(language(&diff, 0), Some("python3"));
        assert_eq!(syntax(&diff, 0), Some("Python"));
        assert_eq!(language(&diff, 1), Some("rust"));

        // "Auto-detect" is not offered, and is harmless if it arrives.
        diff.panes[0].use_auto_detect();
        assert_eq!(language(&diff, 0), Some("python3"));
    }

    // S11 smoke rows: the canonical file of each newly mapped language and
    // the Meld id it detects through the same gate the editor uses. A row is
    // `(what the file is, text, expected id)`, and the name is in every
    // failure message so a broken language is named, not just counted.
    const SMOKE: &[(&str, &str, &str)] = &[
        ("a C program", C, "c"),
        ("a C++ program", CPP, "cpp"),
        ("a C# program", CS, "c-sharp"),
        ("a CSS stylesheet", CSS, "css"),
        ("a shell script", SHELL, "sh"),
        ("a SQL script", SQL, "sql"),
        ("a Ruby class", RUBY, "ruby"),
        ("a PHP class", PHP, "php"),
        ("a Perl script", PERL, "perl"),
        ("a Lua script", LUA, "lua"),
        ("a Haskell module", HASKELL, "haskell"),
        ("an OCaml module", OCAML, "ocaml"),
        ("an Objective-C class", OBJC, "objc"),
        ("an Erlang module", ERLANG, "erlang"),
        ("a Windows batch script", BATCH, "dosbatch"),
        ("an R script", R, "r"),
        ("a Scala program", SCALA, "scala"),
        ("a Common Lisp script", LISP, "commonlisp"),
    ];

    #[test]
    fn every_newly_mapped_language_is_detected_from_its_own_file() {
        for &(name, text, id) in SMOKE {
            assert_eq!(detect(text), Detected::Language(id), "smoke row {name}");
        }
    }

    #[test]
    fn panes_detect_their_own_text() {
        let mut diff = FileDiff::for_test(2);
        paste(&mut diff, 0, JSON);
        paste(&mut diff, 1, YAML);
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(language(&diff, 1), Some("yaml"));

        replace(&mut diff, 1, PROSE);
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(language(&diff, 1), Some("yaml"));

        replace(&mut diff, 1, MARKDOWN);
        assert_eq!(language(&diff, 0), Some("json"));
        assert_eq!(language(&diff, 1), Some("markdown"));
    }
}
