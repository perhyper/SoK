# Structure of Knowledge: Drug informatics

## Structured input

```sok-json sok-knowledge/v1
{
  "schema_version": "sok-knowledge/v1",
  "field": "Drug informatics",
  "elements": [
    {
      "id": "element-drug-label-1111111111",
      "element_class": "Regulatory document",
      "label": {
        "text": "Approved drug labeling",
        "locale": "en"
      },
      "actual_form": {
        "text": "A reviewed document covering indications, dosage and administration, contraindications, and drug interactions",
        "locale": "en"
      },
      "semantic_roles": ["core", "evidence"],
      "role_note": "core",
      "confidence": "high"
    }
  ]
}
```

The JSON block above is machine input and should not appear in the public
narrative body.
