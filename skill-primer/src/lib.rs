use std::fs;
use std::path::{Path, PathBuf};

/// Parsed YAML frontmatter from a SKILL.md file.
#[derive(Debug, serde::Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
}

/// A discovered skill with metadata.
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
}

/// Warnings that can occur during skill scanning.
#[derive(Debug)]
pub enum ScanWarning {
    /// SKILL.md has invalid or missing frontmatter.
    InvalidFrontmatter(PathBuf),
    /// SKILL.md could not be read.
    Unreadable(PathBuf),
    /// Skill name fails validation.
    InvalidName {
        name: String,
        path: PathBuf,
        reason: String,
    },
}

/// The result of scanning one or more skill directories.
pub struct ScanResult {
    pub skills: Vec<Skill>,
    pub warnings: Vec<ScanWarning>,
}

/// Escape special XML characters in a text string.
pub fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Recursively scan a directory for SKILL.md files and extract skill metadata.
///
/// If the directory itself contains a SKILL.md, it is treated as a single skill
/// directory and recursion stops. Otherwise, subdirectories are scanned.
pub fn scan_skill_directory(dir: &Path) -> ScanResult {
    let mut skills = Vec::new();
    let mut warnings = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        let entries: Vec<_> = entries.flatten().collect();

        // Check if this directory is a skill directory (contains SKILL.md)
        let skill_file = entries
            .iter()
            .find(|e| e.path().file_name().is_some_and(|f| f == "SKILL.md"));

        if let Some(entry) = skill_file {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(&path) {
                match parse_skill_frontmatter(&content) {
                    Some(frontmatter) => {
                        // Validate the skill name per spec
                        if let Err(reason) = validate_skill_name(&frontmatter.name) {
                            warnings.push(ScanWarning::InvalidName {
                                name: frontmatter.name.clone(),
                                path: path.clone(),
                                reason,
                            });
                        }
                        skills.push(Skill {
                            name: frontmatter.name,
                            description: frontmatter.description,
                            path,
                        });
                    }
                    None => {
                        // If the file has frontmatter delimiters, parsing was attempted but failed
                        let trimmed = content
                            .strip_prefix("\u{FEFF}")
                            .unwrap_or(&content)
                            .trim_start();
                        if trimmed.starts_with("---") {
                            warnings.push(ScanWarning::InvalidFrontmatter(path));
                        }
                        // Otherwise it's a markdown file without frontmatter — ignore silently
                    }
                }
            } else {
                warnings.push(ScanWarning::Unreadable(path));
            }
            // Do not recurse into subdirectories of a skill directory
            return ScanResult { skills, warnings };
        }

        // Otherwise recurse into subdirectories
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                let sub = scan_skill_directory(&path);
                skills.extend(sub.skills);
                warnings.extend(sub.warnings);
            }
        }
    }

    ScanResult { skills, warnings }
}

/// Validate a skill name against the specification rules.
///
/// Rules:
/// - Must be non-empty
/// - At most 64 characters
/// - Only lowercase ASCII letters, ASCII digits, and hyphens
/// - Must not start or end with a hyphen
/// - Must not contain consecutive hyphens
pub fn validate_skill_name(name: &str) -> Result<(), String> {
    if name.len() > 64 {
        return Err("name exceeds 64 characters".to_string());
    }

    let chars: Vec<char> = name.chars().collect();

    if chars.is_empty() {
        return Err("name is empty".to_string());
    }

    if chars[0] == '-' {
        return Err("name starts with hyphen".to_string());
    }

    if chars[chars.len() - 1] == '-' {
        return Err("name ends with hyphen".to_string());
    }

    let mut prev_hyphen = false;
    for &c in &chars {
        if c == '-' {
            if prev_hyphen {
                return Err("name contains consecutive hyphens".to_string());
            }
            prev_hyphen = true;
        } else if !c.is_ascii_lowercase() && !c.is_ascii_digit() {
            return Err(format!("name contains invalid character '{}'", c));
        } else {
            prev_hyphen = false;
        }
    }

    Ok(())
}

/// Parse YAML frontmatter from a SKILL.md file content.
///
/// Returns `None` if no valid frontmatter is found or if required fields
/// (name, description) are missing or blank.
pub fn parse_skill_frontmatter(content: &str) -> Option<SkillFrontmatter> {
    // Find frontmatter between --- delimiters
    // Strip UTF-8 BOM if present
    let content = content
        .strip_prefix("\u{FEFF}")
        .unwrap_or(content)
        .trim_start();
    if !content.starts_with("---") {
        return None;
    }

    let rest = &content[3..];
    let mut search_start = 0;
    let end = loop {
        match rest[search_start..].find("\n---") {
            Some(pos) => {
                let after = search_start + pos + 4; // after the full "\n---" delimiter
                if after == rest.len() || rest[after..].starts_with('\n') {
                    break search_start + pos;
                }
                search_start += pos + 1;
            }
            None => return None,
        }
    };
    let yaml_content = &rest[..end];
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(yaml_content).ok()?;
    if frontmatter.name.trim().is_empty() || frontmatter.description.trim().is_empty() {
        return None;
    }
    Some(frontmatter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn parse_skill_frontmatter_valid() {
        let content = indoc! {"---
            name: foo
            description: bar
            ---
            # Body
            "};
        let result = parse_skill_frontmatter(content).unwrap();
        assert_eq!(result.name, "foo");
        assert_eq!(result.description, "bar");
    }

    #[test]
    fn parse_skill_frontmatter_missing_delimiters() {
        let content = indoc! {"# No frontmatter
            Just body
            "};
        assert!(parse_skill_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_frontmatter_bad_yaml() {
        let content = indoc! {"---
            name: foo
            description: [unclosed
            ---
            # Body
            "};
        assert!(parse_skill_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_frontmatter_with_utf8_bom() {
        let content = "\u{FEFF}".to_string()
            + indoc! {"---
            name: foo
            description: bar
            ---
            # Body
            "};
        let result = parse_skill_frontmatter(&content).unwrap();
        assert_eq!(result.name, "foo");
        assert_eq!(result.description, "bar");
    }

    #[test]
    fn parse_skill_frontmatter_missing_name() {
        let content = indoc! {"---
            description: bar
            ---
            # Body
            "};
        assert!(parse_skill_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_frontmatter_missing_description() {
        let content = indoc! {"---
            name: foo
            ---
            # Body
            "};
        assert!(parse_skill_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_frontmatter_empty_name() {
        let content = indoc! {"---
            name: \"\"
            description: bar
            ---
            # Body
            "};
        assert!(parse_skill_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_frontmatter_empty_description() {
        let content = indoc! {"---
            name: foo
            description: \"\"
            ---
            # Body
            "};
        assert!(parse_skill_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_frontmatter_whitespace_only() {
        let content = "---\nname: \"   \"\ndescription: \"\t\"\n---\n# Body\n";
        assert!(parse_skill_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_frontmatter_with_end_delimiter_in_value() {
        let content = indoc! {"---
            name: foo
            description: |
              multi-line
              ---
              value
            ---
            # Body
            "};
        let result = parse_skill_frontmatter(content).unwrap();
        assert_eq!(result.name, "foo");
        assert_eq!(result.description, "multi-line\n---\nvalue");
    }
}
