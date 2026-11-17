## {{ tag }} — {{ date }}
{%- for section in sections %}
{%- if section.is_breaking %}

> [!WARNING]
> **Breaking changes**
{%- for c in section.entries %}

> - {{ c.line }}{% if c.breaking_description %}
>   > **Migration**: {{ c.breaking_description }}{% endif %}
{%- endfor %}
{%- else %}

### {{ section.heading }}
{%- for c in section.entries %}
- {{ c.line }}
{%- endfor %}
{%- endif %}
{%- endfor %}
{%- if first_time_contributors %}

### 🌱 First-time contributors
{%- for author in first_time_contributors %}
@{{ author }}{{ " · " if not loop.last }}
{%- endfor %}
{%- endif %}
{%- if contributors %}

### 👥 Contributors
{%- for author in contributors %}
@{{ author }}{{ " · " if not loop.last }}
{%- endfor %}
{%- endif %}
{%- if compare_url %}

[{% if previous_tag %}Compare {{ previous_tag }}...{{ tag }}{% else %}Full changelog{% endif %}]({{ compare_url }})
{%- endif %}
