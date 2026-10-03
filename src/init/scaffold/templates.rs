//! Default template definitions and content constants for `cutver init`.

pub const STARTER_CHANGELOG: &str = "# Changelog

All notable changes to this project will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]
";

pub const DEFAULT_RELEASE_TEMPLATE: &str = r#"## [{{ tag }}] - {{ date }}

{%- if breaking %}
### ⚠️ Breaking Changes
{% for c in commits if c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endif %}

{%- if features %}
### 🚀 Features & Enhancements
{% for c in commits if c.commit_type == 'feat' and not c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endif %}

{%- if fixes %}
### 🐛 Bug Fixes
{% for c in commits if c.commit_type == 'fix' and not c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endif %}

{%- if perf %}
### ⚡ Performance Improvements
{% for c in commits if c.commit_type == 'perf' and not c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endif %}

{%- if refactor %}
### 🔄 Code Refactoring
{% for c in commits if c.commit_type == 'refactor' and not c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endif %}

{%- if docs %}
### 📚 Documentation
{% for c in commits if c.commit_type == 'docs' and not c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endif %}

{%- if maintenance %}
### 🛠️ Maintenance & Dependencies
{% for c in commits if c.commit_type in ['chore', 'build', 'ci', 'test'] and not c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endif %}

{%- if other %}
### 📦 Other Changes
{{ other }}
{%- endif %}

{%- if contributors %}
### 👥 Contributors
{% for author in contributors -%}
- @{{ author }}
{% endfor %}
{%- endif %}

{%- if compare_url %}
---
**Full Changelog**: {{ compare_url }}
{%- endif %}
"#;

pub const DEFAULT_TEMPLATE_PATH: &str = ".github/templates/cutver/RELEASE.md";
