# Structure of Knowledge: 약물정보학

## 기계 구조 입력

```sok-json sok-knowledge/v1
{
  "schema_version": "sok-knowledge/v1",
  "field": "약물정보학",
  "elements": [
    {
      "id": "element-drug-label-1111111111",
      "element_class": "규제 문서",
      "label": {
        "text": "의약품 허가사항",
        "locale": "ko"
      },
      "actual_form": {
        "text": "효능, 용법, 금기, 상호작용을 담은 검토된 문서",
        "locale": "ko"
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
