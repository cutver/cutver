## {{ tag }} — {{ date }}

{%- if breaking %}

> [!WARNING]
> **Breaking changes**
{% for c in commits if c.is_breaking -%}
> - {{ c.line }}
{%- if c.breaking_description %}
>   > **Migration**: {{ c.breaking_description }}
{%- endif %}
{% endfor %}
{%- endif %}

{%- for type, type_commits in commits | group_by_type %}
{%- set visible = type_commits | rejectattr("is_breaking") %}
{%- if visible %}
{%- if type == 'feat' %}
### ✨ Features
{%- elif type == 'fix' %}
### 🐛 Fixes
{%- elif type == 'perf' %}
### ⚡ Performance
{%- elif type == 'refactor' %}
### 🔄 Refactoring
{%- elif type == 'docs' %}
### 📚 Documentation
{%- elif type in ['chore', 'build', 'ci', 'test', 'style', 'revert'] %}
### 🧰 Maintenance
{%- else %}
### {{ type | title }}
{%- endif %}
{% for c in visible -%}
- {{ c.line }}
{% endfor %}
{%- endif %}
{%- endfor %}

{%- if first_time_contributors %}
### 🌱 First-time contributors
{% for author in first_time_contributors -%}
@{{ author }}{{ " · " if not loop.last }}
{%- endfor %}
{%- endif %}

{%- if contributors %}
### 👥 Contributors
{% for author in contributors -%}
@{{ author }}{{ " · " if not loop.last }}
{%- endfor %}
{%- endif %}

{%- if compare_url %}

[{% if previous_tag %}Compare {{ previous_tag }}...{{ tag }}{% else %}Full changelog{% endif %}]({{ compare_url }})
{%- endif %}
