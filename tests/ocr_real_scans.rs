//! Real-scan OCR evaluation (#989).
//!
//! Measures the OCR backends on scanned book pages that have proofread text.
//! The set is pinned in `docs/ocr-eval-manifest.toml`; the protocol and the
//! recorded results are in `docs/ocr-model-metrics.md`.
//!
//! Three layers, by what they need:
//!
//! - [`text`] — wikitext to reference text, and the text normalisation.
//!   Pure functions; their tests run in every `cargo test`.
//! - `set` (feature `ocr-onnx`, for its SHA-256) — the manifest and the
//!   checksum-verified files. Its test skips silently when the set is not
//!   fetched, same policy as the model-gated tests.
//! - `measure` (features `ocr-onnx` and `ocr-tesseract`) — the measurement.
//!   It is `#[ignore]`d: it needs the set, the models and a system Tesseract
//!   with `rus` and `eng` data, and it takes minutes.
//!
//! ```sh
//! bash scripts/fetch_ocr_models.sh
//! bash scripts/fetch_ocr_eval_set.sh
//! cargo test --release --features ocr-onnx,ocr-tesseract \
//!     --test ocr_real_scans -- --ignored --nocapture
//! ```
//!
//! Set `DJVU_OCR_EVAL_DUMP=<dir>` to also write the reference and both
//! recognized texts of every page there, for reading the errors.

mod text {
    /// Convert the wikitext of one Wikisource page to the text that is
    /// printed on the scanned page.
    ///
    /// Keeps everything the proofreaders transcribed, including the running
    /// header and the page number when the page has them (the `noinclude`
    /// blocks). Footnotes (`<ref>`) move to the end, where the page prints
    /// them. Unknown markup is an error, never a guess: a template this
    /// function does not know could hide or invent text.
    pub fn reference_text(wikitext: &str) -> Result<String, String> {
        let s = remove_comments(wikitext)?;
        let (s, notes) = take_refs(&s)?;
        let s = strip_tags(&s)?.replace("__NOEDITSECTION__", "");
        let mut s = expand_templates(&s)?;
        for note in notes {
            s.push('\n');
            s.push_str(&expand_templates(&note)?);
        }
        let s = s.replace("'''''", "").replace("'''", "").replace("''", "");
        for leftover in ["{{", "}}", "[[", "]]", "{|", "|}", "<", ">"] {
            if s.contains(leftover) {
                return Err(format!("unhandled markup `{leftover}`"));
            }
        }
        if let Some(entity) = find_entity(&s) {
            return Err(format!("unhandled character entity `{entity}`"));
        }
        Ok(s)
    }

    fn remove_comments(s: &str) -> Result<String, String> {
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(start) = rest.find("<!--") {
            out.push_str(&rest[..start]);
            let end = rest[start..].find("-->").ok_or("unterminated comment")?;
            rest = &rest[start + end + 3..];
        }
        out.push_str(rest);
        Ok(out)
    }

    /// Cut every `<ref>…</ref>` out of `s`; return the rest and the notes.
    fn take_refs(s: &str) -> Result<(String, Vec<String>), String> {
        let mut out = String::with_capacity(s.len());
        let mut notes = Vec::new();
        let mut rest = s;
        while let Some(start) = rest.find("<ref>") {
            out.push_str(&rest[..start]);
            let body = &rest[start + 5..];
            let end = body.find("</ref>").ok_or("unterminated <ref>")?;
            notes.push(body[..end].to_string());
            rest = &body[end + 6..];
        }
        out.push_str(rest);
        Ok((out, notes))
    }

    /// Drop the known tags and keep their content. Block tags become a line
    /// break so the text on both sides does not run together.
    fn strip_tags(s: &str) -> Result<String, String> {
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(start) = rest.find('<') {
            out.push_str(&rest[..start]);
            let end = rest[start..].find('>').ok_or("unterminated tag")?;
            let tag = &rest[start + 1..start + end];
            let name = tag
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("");
            match name {
                "noinclude" | "div" => out.push('\n'),
                "pagequality" | "references" => {}
                _ => return Err(format!("unknown tag <{tag}>")),
            }
            rest = &rest[start + end + 1..];
        }
        out.push_str(rest);
        Ok(out)
    }

    /// Replace every `{{template}}` by what it prints, innermost first.
    fn expand_templates(s: &str) -> Result<String, String> {
        let mut s = s.to_string();
        while let Some(end) = s.find("}}") {
            let start = s[..end].rfind("{{").ok_or("`}}` without `{{`")?;
            let printed = template(&s[start + 2..end])?;
            s.replace_range(start..end + 2, &printed);
        }
        Ok(s)
    }

    fn template(inner: &str) -> Result<String, String> {
        let mut parts = inner.split('|');
        let name = parts.next().unwrap_or("").trim();
        let args: Vec<&str> = parts.collect();
        let arg = |i: usize| -> Result<&str, String> {
            args.get(i)
                .copied()
                .ok_or_else(|| format!("template `{name}` lacks argument {}", i + 1))
        };
        Ok(match name {
            // Layout and decoration: paragraph control, rules, asterisms,
            // the footnote list (the notes themselves come from `<ref>`).
            "нп" | "nop" | "---" | "3*d" | "smallrefs" => String::new(),
            // A thin space after a dialogue dash.
            "fsp" => " ".to_string(),
            // Inline styling.
            "sc" | "x-larger" => arg(0)?.to_string(),
            // Styled blocks: centred lines, headings, right-aligned lines.
            "c" | "center" | "ph" | "right2" => format!("\n{}\n", arg(0)?),
            "roman" => {
                let n = arg(0)?.trim();
                roman(n.parse().map_err(|_| format!("roman: bad number `{n}`"))?)
            }
            "RunningHeader" => {
                let cells: Vec<&str> = args.iter().map(|a| a.trim()).collect();
                let cells: Vec<&str> = cells.into_iter().filter(|a| !a.is_empty()).collect();
                format!("\n{}\n", cells.join(" "))
            }
            // A word split across two pages. The page that ends with the
            // word prints its first part and a hyphen; the next page prints
            // the second part.
            "Перенос" | "hyphenated word start" => format!("{}-", arg(0)?),
            "Перенос2" => arg(1)?.to_string(),
            "hyphenated word end" => arg(0)?.to_string(),
            _ => return Err(format!("unknown template `{name}`")),
        })
    }

    fn roman(mut n: u32) -> String {
        const STEPS: [(u32, &str); 13] = [
            (1000, "M"),
            (900, "CM"),
            (500, "D"),
            (400, "CD"),
            (100, "C"),
            (90, "XC"),
            (50, "L"),
            (40, "XL"),
            (10, "X"),
            (9, "IX"),
            (5, "V"),
            (4, "IV"),
            (1, "I"),
        ];
        let mut out = String::new();
        for (value, digits) in STEPS {
            while n >= value {
                out.push_str(digits);
                n -= value;
            }
        }
        out
    }

    /// The first `&name;` or `&#123;` in `s`, if any.
    fn find_entity(s: &str) -> Option<&str> {
        let mut rest = s;
        while let Some(start) = rest.find('&') {
            let tail = &rest[start + 1..];
            let len = tail
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '#'))
                .unwrap_or(tail.len());
            if len > 0 && tail[len..].starts_with(';') {
                return Some(&rest[start..start + len + 2]);
            }
            rest = tail;
        }
        None
    }

    const HYPHENS: [char; 4] = ['-', '\u{2010}', '\u{2011}', '\u{00AD}'];

    /// Normalise a text before comparison. The reference and the recognized
    /// text both go through this function, so no rule favours a backend.
    ///
    /// 1. A word hyphenated at a line end is joined: a line that ends with a
    ///    letter and a hyphen, followed by a line that starts with a
    ///    lowercase letter. The proofread text has such words joined.
    /// 2. Typographic variants fold to one character: all dashes to `-`,
    ///    all quotation marks to `'` or `"`, `…` to `...`, Latin ligatures
    ///    to their letters.
    /// 3. Spaces next to a dash are dropped. Old print sets dashes close;
    ///    the proofread text spaces them by today's convention.
    /// 4. Every run of whitespace, line breaks included, becomes one space.
    ///
    /// Case, punctuation and digits are compared as they are.
    pub fn normalise(text: &str) -> String {
        // 1. Join words hyphenated at a line end.
        let mut joined = String::with_capacity(text.len());
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
        let mut current = lines.next().unwrap_or("").to_string();
        for next in lines {
            let mut tail = current.chars().rev();
            let ends_hyphenated = tail.next().is_some_and(|c| HYPHENS.contains(&c))
                && tail.next().is_some_and(char::is_alphabetic);
            let starts_lowercase = next.chars().next().is_some_and(char::is_lowercase);
            if ends_hyphenated && starts_lowercase {
                current.pop();
            } else {
                joined.push_str(&current);
                joined.push(' ');
                current.clear();
            }
            current.push_str(next);
        }
        joined.push_str(&current);

        // 2. Fold typographic variants.
        let mut folded = String::with_capacity(joined.len());
        for c in joined.chars() {
            match c {
                '\u{00AD}' | '\u{2010}'..='\u{2015}' | '\u{2212}' => folded.push('-'),
                '\u{2018}'..='\u{201B}' | '\u{2032}' | '`' | '\u{00B4}' => folded.push('\''),
                '\u{201C}'..='\u{201F}' | '\u{2033}' | '«' | '»' => folded.push('"'),
                '…' => folded.push_str("..."),
                'ﬀ' => folded.push_str("ff"),
                'ﬁ' => folded.push_str("fi"),
                'ﬂ' => folded.push_str("fl"),
                'ﬃ' => folded.push_str("ffi"),
                'ﬄ' => folded.push_str("ffl"),
                c => folded.push(c),
            }
        }

        // 3 and 4. One space between words, none next to a dash.
        let mut out = String::with_capacity(folded.len());
        for word in folded.split_whitespace() {
            if !(out.is_empty() || out.ends_with('-') || word.starts_with('-')) {
                out.push(' ');
            }
            out.push_str(word);
        }
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn russian_page_keeps_printed_text_only() {
            let wiki = "<noinclude><pagequality level=\"4\" user=\"X\" />__NOEDITSECTION__\n\
                <div class=\"indent\"></noinclude>{{Перенос2|ко|нец}} слова.\n\n\
                —{{fsp}}Да, — сказал он.\n\n{{3*d}}\n'''1 мая.''' Школа {{roman|14}} \
                ступени. Дальше {{Перенос|на|чало}}<noinclude><!-- -->\n\
                <references /></div></noinclude>";
            let text = reference_text(wiki).unwrap();
            assert_eq!(
                normalise(&text),
                "нец слова.-Да,-сказал он. 1 мая. Школа XIV ступени. Дальше на-"
            );
        }

        #[test]
        fn english_page_keeps_header_footer_and_footnote() {
            let wiki = "<noinclude><pagequality level=\"4\" user=\"X\" />\
                {{RunningHeader||THE TITLE|9}}</noinclude>{{hyphenated word end|pender|suspender}} \
                and more.\n\n{{ph|A Heading.<ref>The note.</ref>}}\n\n{{sc|The}} text \
                {{hyphenated word start|sus|suspender}}<noinclude>{{c|12}}\n\
                {{smallrefs|rule=yes}}</noinclude>";
            let text = reference_text(wiki).unwrap();
            assert_eq!(
                normalise(&text),
                "THE TITLE 9 pender and more. A Heading. The text sus-12 The note."
            );
        }

        #[test]
        fn unknown_markup_is_an_error() {
            assert!(
                reference_text("a {{mystery|b}} c")
                    .unwrap_err()
                    .contains("mystery")
            );
            assert!(
                reference_text("a <poem>b</poem>")
                    .unwrap_err()
                    .contains("poem")
            );
            assert!(reference_text("a [[link]] b").unwrap_err().contains("[["));
            assert!(reference_text("a&nbsp;b").unwrap_err().contains("&nbsp;"));
            assert!(reference_text("Smith & Sons; more").is_ok());
        }

        #[test]
        fn normalise_joins_line_end_hyphenation() {
            assert_eq!(normalise("сирот-\nливые перья"), "сиротливые перья");
            // Not a split word: the next line starts with a capital or a digit.
            assert_eq!(normalise("Нью-\nЙорк"), "Нью-Йорк");
            assert_eq!(normalise("в 1925-\n26 годах"), "в 1925-26 годах");
            // A dash is not a hyphen: nothing is joined.
            assert_eq!(normalise("я—\nначальник"), "я-начальник");
        }

        #[test]
        fn normalise_folds_typography_and_whitespace() {
            assert_eq!(
                normalise("«Да»,  — сказал\u{00A0}он…\n\n“No” — it’s ﬁne."),
                "\"Да\",-сказал он... \"No\"-it's fine."
            );
            assert_eq!(normalise("  \n "), "");
        }

        #[test]
        fn roman_numerals() {
            assert_eq!(roman(1), "I");
            assert_eq!(roman(4), "IV");
            assert_eq!(roman(1926), "MCMXXVI");
        }
    }
}

#[cfg(feature = "ocr-onnx")]
mod set {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use djvu_rs::ocr_onnx::manifest::sha256_hex;

    const MANIFEST: &str = include_str!("../docs/ocr-eval-manifest.toml");

    /// One file of the set: where it is and what its bytes must be.
    pub struct Pinned {
        pub file: String,
        pub size: usize,
        pub sha256: String,
    }

    pub struct Scan {
        pub name: String,
        pub pinned: Pinned,
        pub script: String,
        pub tesseract_lang: String,
    }

    pub struct Page {
        pub scan: String,
        /// 1-based page number in the DjVu file.
        pub page: usize,
        pub pinned: Pinned,
    }

    pub struct Manifest {
        pub scans: Vec<Scan>,
        pub pages: Vec<Page>,
    }

    impl Manifest {
        /// Parse the checked-in manifest. Panics on a malformed file: the
        /// manifest is part of the test.
        pub fn builtin() -> Self {
            let mut tables: Vec<(String, BTreeMap<String, String>)> = Vec::new();
            for line in MANIFEST.lines().map(str::trim) {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(table) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
                    tables.push((table.to_string(), BTreeMap::new()));
                    continue;
                }
                let (key, value) = line
                    .split_once('=')
                    .expect("manifest line is `key = value`");
                let fields = &mut tables.last_mut().expect("key before a table").1;
                fields.insert(
                    key.trim().to_string(),
                    value.trim().trim_matches('"').to_string(),
                );
            }
            let field = |fields: &BTreeMap<String, String>, key: &str| -> String {
                fields
                    .get(key)
                    .unwrap_or_else(|| panic!("manifest entry lacks `{key}`"))
                    .clone()
            };
            let pinned = |fields: &BTreeMap<String, String>| Pinned {
                file: field(fields, "file"),
                size: field(fields, "size").parse().expect("size is a number"),
                sha256: field(fields, "sha256"),
            };
            let mut manifest = Manifest {
                scans: Vec::new(),
                pages: Vec::new(),
            };
            for (table, fields) in &tables {
                match table.as_str() {
                    "scan" => manifest.scans.push(Scan {
                        name: field(fields, "name"),
                        pinned: pinned(fields),
                        script: field(fields, "script"),
                        tesseract_lang: field(fields, "tesseract_lang"),
                    }),
                    "page" => manifest.pages.push(Page {
                        scan: field(fields, "scan"),
                        page: field(fields, "page").parse().expect("page is a number"),
                        pinned: pinned(fields),
                    }),
                    other => panic!("unknown manifest table [[{other}]]"),
                }
            }
            manifest
        }
    }

    /// The set directory: `$DJVU_OCR_EVAL_DIR` if set, else `models/ocr-eval`
    /// in the repository (where `scripts/fetch_ocr_eval_set.sh` puts it).
    pub fn eval_dir() -> PathBuf {
        std::env::var_os("DJVU_OCR_EVAL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("models/ocr-eval"))
    }

    /// Read a pinned file and refuse it unless size and SHA-256 match.
    pub fn load_verified(dir: &Path, pinned: &Pinned) -> Result<Vec<u8>, String> {
        let path = dir.join(&pinned.file);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if bytes.len() != pinned.size {
            return Err(format!(
                "{}: size {} != pinned {}",
                pinned.file,
                bytes.len(),
                pinned.size
            ));
        }
        let got = sha256_hex(&bytes);
        if got != pinned.sha256 {
            return Err(format!(
                "{}: sha256 {got} != pinned {}",
                pinned.file, pinned.sha256
            ));
        }
        Ok(bytes)
    }

    /// The normalised reference text of one page.
    pub fn reference(dir: &Path, page: &Page) -> Result<String, String> {
        let bytes = load_verified(dir, &page.pinned)?;
        let wikitext =
            String::from_utf8(bytes).map_err(|e| format!("{}: {e}", page.pinned.file))?;
        let text = super::text::reference_text(&wikitext)
            .map_err(|e| format!("{}: {e}", page.pinned.file))?;
        Ok(super::text::normalise(&text))
    }

    #[test]
    fn manifest_is_consistent() {
        let manifest = Manifest::builtin();
        assert_eq!(manifest.scans.len(), 4);
        for script in ["cyrillic", "latin"] {
            let scans: Vec<&Scan> = manifest
                .scans
                .iter()
                .filter(|s| s.script == script)
                .collect();
            assert_eq!(scans.len(), 2, "two scans per script");
            assert!(scans.iter().all(|s| !s.tesseract_lang.is_empty()));
        }
        for scan in &manifest.scans {
            let pages = manifest
                .pages
                .iter()
                .filter(|p| p.scan == scan.name)
                .count();
            assert_eq!(pages, 5, "five pages for {}", scan.name);
        }
        assert_eq!(manifest.pages.len(), 20, "no page without its scan");
        assert!(manifest.pages.iter().all(|p| p.page >= 1));
    }

    /// Every fetched file matches its pin and every reference page converts.
    /// Skips when the set is not fetched. With `DJVU_OCR_EVAL_DUMP` set, it
    /// writes the reference texts there; no OCR runs.
    #[test]
    fn fetched_set_verifies_and_converts() {
        let manifest = Manifest::builtin();
        let dir = eval_dir();
        if !dir.join(&manifest.scans[0].pinned.file).exists() {
            return; // set not fetched — skip
        }
        for scan in &manifest.scans {
            load_verified(&dir, &scan.pinned).unwrap();
        }
        let dump = std::env::var_os("DJVU_OCR_EVAL_DUMP").map(PathBuf::from);
        for page in &manifest.pages {
            let text = reference(&dir, page).unwrap();
            if let Some(dump) = &dump {
                std::fs::create_dir_all(dump).unwrap();
                let stem = page.pinned.file.trim_end_matches(".wiki");
                std::fs::write(dump.join(format!("{stem}.reference.txt")), &text).unwrap();
            }
            assert!(
                text.chars().count() > 300,
                "{} is too short",
                page.pinned.file
            );
        }
    }
}

#[cfg(all(feature = "ocr-onnx", feature = "ocr-tesseract"))]
mod measure {
    use std::time::Instant;

    use djvu_rs::{
        DjVuDocument,
        djvu_render::{RenderOptions, render_pixmap},
        ocr::{OcrBackend, OcrOptions},
        ocr_onnx::{metrics::levenshtein, pipeline::NeuralOcrBackend},
        ocr_tesseract::TesseractBackend,
    };

    use super::set::{Manifest, eval_dir, load_verified, reference};
    use super::text::normalise;

    const BACKENDS: [&str; 2] = ["ocr-onnx", "ocr-tesseract"];

    /// Edit distances and reference sizes, summed over pages. Rates are
    /// pooled: total distance over total reference length.
    #[derive(Default, Clone)]
    struct Score {
        char_errors: usize,
        chars: usize,
        word_errors: usize,
        words: usize,
        seconds: Vec<f64>,
    }

    impl Score {
        fn add(&mut self, other: &Score) {
            self.char_errors += other.char_errors;
            self.chars += other.chars;
            self.word_errors += other.word_errors;
            self.words += other.words;
            self.seconds.extend_from_slice(&other.seconds);
        }

        fn cer(&self) -> f64 {
            self.char_errors as f64 / self.chars as f64
        }

        fn wer(&self) -> f64 {
            self.word_errors as f64 / self.words as f64
        }

        fn mean_seconds(&self) -> f64 {
            self.seconds.iter().sum::<f64>() / self.seconds.len() as f64
        }

        fn median_seconds(&self) -> f64 {
            let mut sorted = self.seconds.clone();
            sorted.sort_by(f64::total_cmp);
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 1 {
                sorted[mid]
            } else {
                (sorted[mid - 1] + sorted[mid]) / 2.0
            }
        }
    }

    fn score(reference: &str, hypothesis: &str, seconds: f64) -> Score {
        let r: Vec<char> = reference.chars().collect();
        let h: Vec<char> = hypothesis.chars().collect();
        let rw: Vec<&str> = reference.split_whitespace().collect();
        let hw: Vec<&str> = hypothesis.split_whitespace().collect();
        Score {
            char_errors: levenshtein(&r, &h),
            chars: r.len(),
            word_errors: levenshtein(&rw, &hw),
            words: rw.len(),
            seconds: vec![seconds],
        }
    }

    #[test]
    #[ignore = "needs the evaluation set, the OCR models and Tesseract with rus+eng data"]
    fn measure_real_scans() {
        let manifest = Manifest::builtin();
        let dir = eval_dir();
        let dump = std::env::var_os("DJVU_OCR_EVAL_DUMP").map(std::path::PathBuf::from);
        if let Some(dump) = &dump {
            std::fs::create_dir_all(dump).unwrap();
        }

        let onnx = NeuralOcrBackend::load_default().expect("run scripts/fetch_ocr_models.sh");
        let tesseract = TesseractBackend::new();
        let backends: [&dyn OcrBackend; 2] = [&onnx, &tesseract];

        // per_scan[scan][backend], in manifest order.
        let mut per_scan: Vec<[Score; 2]> = Vec::new();

        println!("\n### Per page\n");
        println!(
            "| Scan | Page | Reference chars | onnx CER | onnx WER | onnx s | tesseract CER | tesseract WER | tesseract s |"
        );
        println!(
            "|------|------|-----------------|----------|----------|--------|---------------|---------------|-------------|"
        );
        for scan in &manifest.scans {
            let bytes =
                load_verified(&dir, &scan.pinned).expect("run scripts/fetch_ocr_eval_set.sh");
            let doc = DjVuDocument::parse(&bytes).unwrap();
            let mut totals = [Score::default(), Score::default()];
            for page in manifest.pages.iter().filter(|p| p.scan == scan.name) {
                let reference = reference(&dir, page).unwrap();
                let djvu_page = doc.page(page.page - 1).unwrap();
                // Same render as `djvu ocr`: the page's native resolution.
                let render = RenderOptions {
                    width: djvu_page.width() as u32,
                    height: djvu_page.height() as u32,
                    ..Default::default()
                };
                let pixmap = render_pixmap(djvu_page, &render).unwrap();
                let options = OcrOptions {
                    languages: scan.tesseract_lang.clone(),
                    dpi: djvu_page.dpi() as u32,
                };
                let stem = page.pinned.file.trim_end_matches(".wiki");
                if let Some(dump) = &dump {
                    std::fs::write(dump.join(format!("{stem}.reference.txt")), &reference).unwrap();
                }
                let mut row = Vec::new();
                for (i, backend) in backends.iter().enumerate() {
                    let start = Instant::now();
                    let layer = backend.recognize(&pixmap, &options).unwrap();
                    let seconds = start.elapsed().as_secs_f64();
                    let hypothesis = normalise(&layer.text);
                    if let Some(dump) = &dump {
                        let name = BACKENDS[i];
                        std::fs::write(dump.join(format!("{stem}.{name}.raw.txt")), &layer.text)
                            .unwrap();
                        std::fs::write(dump.join(format!("{stem}.{name}.txt")), &hypothesis)
                            .unwrap();
                    }
                    let s = score(&reference, &hypothesis, seconds);
                    row.push(format!("{:.4} | {:.4} | {:.2}", s.cer(), s.wer(), seconds));
                    totals[i].add(&s);
                }
                println!(
                    "| {} | {} | {} | {} |",
                    scan.name,
                    page.page,
                    reference.chars().count(),
                    row.join(" | ")
                );
            }
            per_scan.push(totals);
        }

        let print_rows = |title: &str, rows: &[(String, [Score; 2])]| {
            println!("\n### {title}\n");
            println!(
                "| Set | Pages | Reference chars | Backend | CER | WER | Median s/page | Mean s/page |"
            );
            println!(
                "|-----|-------|-----------------|---------|-----|-----|---------------|-------------|"
            );
            for (name, scores) in rows {
                for (i, s) in scores.iter().enumerate() {
                    println!(
                        "| {name} | {} | {} | {} | {:.4} | {:.4} | {:.2} | {:.2} |",
                        s.seconds.len(),
                        s.chars,
                        BACKENDS[i],
                        s.cer(),
                        s.wer(),
                        s.median_seconds(),
                        s.mean_seconds()
                    );
                }
            }
        };

        let scan_rows: Vec<(String, [Score; 2])> = manifest
            .scans
            .iter()
            .zip(&per_scan)
            .map(|(scan, scores)| (scan.name.clone(), scores.clone()))
            .collect();
        print_rows("Per scan", &scan_rows);

        let mut script_rows: Vec<(String, [Score; 2])> = Vec::new();
        for (scan, scores) in manifest.scans.iter().zip(&per_scan) {
            let position = script_rows
                .iter()
                .position(|(name, _)| *name == scan.script);
            let index = position.unwrap_or_else(|| {
                script_rows.push((scan.script.clone(), [Score::default(), Score::default()]));
                script_rows.len() - 1
            });
            for (total, s) in script_rows[index].1.iter_mut().zip(scores) {
                total.add(s);
            }
        }
        print_rows(
            "Per script (the decision is made on these rows)",
            &script_rows,
        );
    }
}
