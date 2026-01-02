## Output Requirements

When you reply:

- Output **only** the complete `contract.yaml` content.
- Wrap it in a fenced code block like:

```yaml
# contract.yaml
schema_version: 1
metadata:
  ...
# (rest of contract here)
```

Do not include any prose, commentary, or explanation outside the YAML.

Ensure the YAML is:

- Well-formed and parseable (e.g. by serde_yaml).
- Internally consistent with the feature spec, Constitution, and ADRs.
- Aligned with the existing templates in `docs/templates/` (minimal vs critical, required sections, field names).

If anything is ambiguous, choose the safest conservative behaviour and document it clearly in the `requirements`, `assumptions`, and `logic.error_conditions` sections of the contract.
