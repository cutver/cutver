//! The 1.0 freeze classifies every judgement obligation, and this file proves the classification is
//! complete instead of declared (#353; `CONTRACT.md`, `## What the 1.0 Freeze Covers`).
//!
//! Why a pin and not a promise: the reachability condition this serves reads "the enforcement table
//! has no `judgement` row left in anything the 1.0 promises", and this project's failure mode is a
//! declaration nobody wired to a check. The classification itself stays the maintainer's decision;
//! what is mechanical is that no judgement obligation can sit outside it and that no row can name an
//! obligation the contract does not carry.
//!
//! The pin derives obligations from the two shapes this contract states them in:
//!
//! 1. the `Judgement` row of the limits table, keyed by the limit it names;
//! 2. every judgement-worded clause inside a pillar, keyed by the bullet's bold lead, or by the
//!    subsection title when the clause stands on its own.
//!
//! Both directions are compared, and the walk asserts a floor, so a pattern that stops matching
//! cannot make the contract look clean.

use std::path::PathBuf;

const DOC: &str = "CONTRACT.md";
const SECTION: &str = "## What the 1.0 Freeze Covers";
const LIMITS_HEADER: &str = "| Limit | Level | Where it is checked |";
const LEVEL: &str = "judgement";
/// The contract carries six judgement locations today, which fold into five obligations. A scan that
/// found fewer than five has lost its patterns, not cleaned the contract.
const MINIMUM_OBLIGATIONS: usize = 5;
/// A classification without its reason is the checklist this pin exists to prevent.
const MINIMUM_REASON_CHARS: usize = 20;
const VERDICTS: [&str; 2] = ["inside", "outside"];

fn document() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DOC);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{} must be readable: {err}", path.display()))
}

/// Compares what two sentences say rather than how they are written: the list and the clause it
/// names are prose from different hands, so backticks, emphasis, spacing and a trailing period are
/// not part of an obligation's identity.
fn fold(text: &str) -> String {
    let plain: String = text
        .chars()
        .filter(|character| *character != '`' && *character != '*')
        .collect();
    plain
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .trim_end_matches(['.', ':'])
        .to_ascii_lowercase()
}

/// One place in the contract that states a judgement obligation.
#[derive(Debug, PartialEq, Eq)]
struct Location {
    key: String,
    line: usize,
}

impl std::fmt::Display for Location {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "`{}` at {DOC}:{}", self.key, self.line)
    }
}

/// The bold lead of a bullet, which is how this contract names the rule it states.
fn bold_lead(line: &str) -> Option<String> {
    let rest = line.trim_start_matches(['-', '*']).trim_start();
    let start = rest.find("**")? + 2;
    let tail = &rest[start..];
    let end = tail.find("**")?;
    Some(tail[..end].to_string())
}

/// A subsection title without its ordinal: `### 4. Atomic Primitives` identifies as
/// `Atomic Primitives`, because numbering is presentation and the pin compares identity.
fn without_ordinal(title: &str) -> String {
    let trimmed = title.trim();
    match trimmed.split_once('.') {
        Some((ordinal, rest)) if ordinal.chars().all(|character| character.is_ascii_digit()) => rest.trim().to_string(),
        _ => trimmed.to_string(),
    }
}

fn locations(text: &str) -> Vec<Location> {
    let mut found: Vec<Location> = Vec::new();
    let mut in_pillar = false;
    let mut subsection = String::new();
    let mut bullet: Option<String> = None;
    let mut in_limits_table = false;

    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_start();
        let number = index + 1;

        if line.starts_with("## ") {
            in_pillar = line.contains("Pillar") && !line.starts_with(SECTION);
            subsection.clear();
            bullet = None;
            in_limits_table = false;
            continue;
        }
        if line.starts_with("### ") {
            subsection = without_ordinal(line.trim_start_matches('#'));
            bullet = None;
            continue;
        }
        if line.starts_with("| ") {
            if line.starts_with(LIMITS_HEADER) {
                in_limits_table = true;
                continue;
            }
            if in_limits_table {
                let cells: Vec<&str> = line.split('|').collect();
                if cells.len() > 2 && cells[2].to_ascii_lowercase().contains(LEVEL) {
                    found.push(Location {
                        key: fold(cells[1]),
                        line: number,
                    });
                }
                continue;
            }
            if in_pillar && line.to_ascii_lowercase().contains(LEVEL) {
                found.push(Location {
                    key: fold(&subsection),
                    line: number,
                });
            }
            continue;
        }
        in_limits_table = false;

        if line.is_empty() {
            bullet = None;
            continue;
        }
        if let Some(lead) = bold_lead(line).filter(|_| line.starts_with("- ") || line.starts_with("* ")) {
            bullet = Some(lead);
        }
        if in_pillar && line.to_ascii_lowercase().contains(LEVEL) {
            let key = bullet.clone().unwrap_or_else(|| subsection.clone());
            assert!(
                !key.is_empty(),
                "a judgement clause at {DOC}:{number} belongs to no pillar subsection and no bullet, so this \
                 pin cannot name it; give it one or extend the pin's derivation"
            );
            found.push(Location {
                key: fold(&key),
                line: number,
            });
        }
    }

    found
}

/// One row of the classification list.
#[derive(Debug)]
struct Entry {
    obligation: String,
    locations: Vec<String>,
    verdict: String,
    reason: String,
}

fn entries(text: &str) -> Vec<Entry> {
    let mut rows = Vec::new();
    let mut in_section = false;
    let mut in_table = false;

    for raw in text.lines() {
        let line = raw.trim_start();
        if line.starts_with("## ") {
            in_section = line.starts_with(SECTION);
            in_table = false;
            continue;
        }
        if !in_section || !line.starts_with("| ") {
            continue;
        }
        if line.starts_with("| Judgement obligation |") {
            in_table = true;
            continue;
        }
        if !in_table || line.contains(":---") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').collect();
        if cells.len() < 6 {
            continue;
        }
        rows.push(Entry {
            obligation: cells[1].trim().to_string(),
            locations: cells[2].split('`').skip(1).step_by(2).map(fold).collect(),
            verdict: fold(cells[3]),
            reason: cells[4].trim().to_string(),
        });
    }

    rows
}

/// Compares the two sets in both directions. `Ok` carries how many locations the contract states.
fn compare(text: &str) -> Result<usize, String> {
    let found = locations(text);
    let listed = entries(text);

    let unclassified: Vec<String> = found
        .iter()
        .filter(|location| {
            !listed
                .iter()
                .any(|entry| entry.locations.iter().any(|key| key == &location.key))
        })
        .map(ToString::to_string)
        .collect();
    if !unclassified.is_empty() {
        return Err(format!(
            "every judgement obligation in {DOC} must be classified in `{SECTION}`, with its verdict and its \
             reason (#353). Unclassified: {unclassified:?}"
        ));
    }

    let mut invented: Vec<String> = Vec::new();
    for entry in &listed {
        if entry.locations.is_empty() {
            return Err(format!(
                "the classification row `{}` names no obligation; the pin compares locations, so a row without \
                 one is unverifiable (#353)",
                entry.obligation
            ));
        }
        for key in &entry.locations {
            if !found.iter().any(|location| &location.key == key) {
                invented.push(format!("`{key}` (named by \"{}\")", entry.obligation));
            }
        }
    }
    if !invented.is_empty() {
        return Err(format!(
            "the classification list in `{SECTION}` may only name obligations {DOC} actually carries, or it \
             drifts into decoration as the contract changes (#353). Invented: {invented:?}"
        ));
    }

    for entry in &listed {
        if !VERDICTS.contains(&entry.verdict.as_str()) {
            return Err(format!(
                "the classification row `{}` must say `Inside` or `Outside`; it says `{}` (#353)",
                entry.obligation, entry.verdict
            ));
        }
        if entry.reason.chars().count() < MINIMUM_REASON_CHARS {
            return Err(format!(
                "the classification row `{}` needs its reason: `{}` is shorter than {MINIMUM_REASON_CHARS} \
                 characters (#353)",
                entry.obligation, entry.reason
            ));
        }
    }

    Ok(found.len())
}

#[test]
fn every_judgement_obligation_is_classified() {
    let text = document();
    let found = locations(&text).len();
    assert!(
        found >= MINIMUM_OBLIGATIONS,
        "the pin must find this contract's judgement obligations before it can prove they are classified; it \
         found {found}, so its patterns have stopped matching rather than the contract having gone clean (#353)"
    );

    let compared = compare(&text).unwrap_or_else(|report| panic!("{report}"));
    assert_eq!(
        compared, found,
        "the comparison must account for every location it found"
    );
}

#[test]
fn an_unclassified_judgement_clause_is_reported_with_its_line() {
    let scratch = concat!(
        "## ⚖️ Enforcement\n",
        "| Level | Means |\n",
        "| :--- | :--- |\n",
        "| **Judgement** | No machine can decide it |\n",
        "## 🏛️ Pillar I: Rules\n",
        "### 1. One Rule\n",
        "- **A rule nobody check can read.** Something true. **Judgement**: review owns it.\n",
        "## What the 1.0 Freeze Covers\n",
        "| Judgement obligation | Where it lives | In the freeze | Why |\n",
        "| :--- | :--- | :--- | :--- |\n",
    );

    let report = compare(scratch).expect_err("an unclassified judgement clause must fail the pin");
    assert!(
        report.contains("a rule nobody check can read") && report.contains("CONTRACT.md:7"),
        "the report must name the obligation and where it lives: {report}"
    );
    assert!(
        report.contains("Unclassified"),
        "the report must say what is wrong in the contract's own terms: {report}"
    );
}

#[test]
fn a_classification_naming_an_absent_obligation_is_reported() {
    let scratch = concat!(
        "## 🏛️ Pillar I: Rules\n",
        "### 1. One Rule\n",
        "- **A rule.** Something true.\n",
        "## What the 1.0 Freeze Covers\n",
        "| Judgement obligation | Where it lives | In the freeze | Why |\n",
        "| :--- | :--- | :--- | :--- |\n",
        "| A ghost | `a ghost clause` | **Outside** | A reason long enough to clear the minimum length. |\n",
    );

    let report = compare(scratch).expect_err("a classification naming an absent obligation must fail the pin");
    assert!(
        report.contains("a ghost clause") && report.contains("Invented"),
        "the report must name the invented obligation and say what is wrong: {report}"
    );
}

#[test]
fn the_derivation_reads_the_limits_table_and_ignores_the_level_vocabulary() {
    let scratch = concat!(
        "## ⚖️ Enforcement\n",
        "| Level | Means | What the clause must name |\n",
        "| :--- | :--- | :--- |\n",
        "| **Judgement** | No machine can decide it | Who decides |\n",
        "| Limit | Level | Where it is checked |\n",
        "| :--- | :--- | :--- |\n",
        "| 500 lines per file | **Judgement** | Review |\n",
    );

    let found = locations(scratch);
    assert_eq!(
        found,
        vec![Location {
            key: "500 lines per file".to_string(),
            line: 7
        }],
        "the vocabulary table defines the level; only the limits table states an obligation"
    );
}
