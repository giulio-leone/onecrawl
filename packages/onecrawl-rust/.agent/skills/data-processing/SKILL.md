---
name: data-processing
description: "Data pipeline engine for filtering, transforming, deduplicating, and exporting scraped data. Supports CSV, JSON, JSONL export with configurable pipeline steps."
---

# Data Processing Skill

Transform, filter, deduplicate, and export scraped data using OneCrawl's configurable pipeline engine.

## Modules

| Module | Purpose |
|--------|---------|
| `data_pipeline` | 9 step types: Filter, Transform, Rename, Remove, Add, Deduplicate, Sort, Limit, Skip |
| `http_client` | Browser-session HTTP client via fetch API (inherits cookies/auth) |
| `cookie_jar` | Persistent cookie storage across sessions |

## How It Works

### Data Pipeline
```bash
# Define pipeline
cat > pipeline.json << 'EOF'
{
  "name": "clean-products",
  "steps": [
    {"Filter": {"field": "price", "operator": "not_empty", "value": ""}},
    {"Transform": {"field": "name", "transform": "trim"}},
    {"Transform": {"field": "price", "transform": "strip_whitespace"}},
    {"Deduplicate": {"field": "url"}},
    {"Sort": {"field": "price", "descending": false}},
    {"Limit": {"count": 100}}
  ]
}
EOF

# Run pipeline
onecrawl pipeline run pipeline.json data.json
onecrawl pipeline run pipeline.json data.json --output clean.csv --format csv
onecrawl pipeline run pipeline.json data.json --output clean.jsonl --format jsonl

# Validate pipeline config
onecrawl pipeline validate pipeline.json
```

### Pipeline Step Types

| Step | Description | Example |
|------|-------------|---------|
| `Filter` | Keep/remove items by field value | `{"field":"status","operator":"eq","value":"active"}` |
| `Transform` | Modify field values | `{"field":"name","transform":"lowercase"}` |
| `Rename` | Rename a field | `{"from":"old_name","to":"new_name"}` |
| `Remove` | Remove fields | `{"fields":["temp","debug"]}` |
| `Add` | Add a constant field | `{"field":"source","value":"onecrawl"}` |
| `Deduplicate` | Remove duplicates by field | `{"field":"url"}` |
| `Sort` | Sort by field value | `{"field":"price","descending":false}` |
| `Limit` | Keep first N items | `{"count":50}` |
| `Skip` | Skip first N items | `{"count":10}` |

### Filter Operators
`eq`, `neq`, `contains`, `starts_with`, `ends_with`, `gt`, `lt`, `empty`, `not_empty`

### Transform Functions
`trim`, `lowercase`, `uppercase`, `strip_html`, `strip_whitespace`, `truncate:N`, `replace:old:new`, `split:delim:index`

### HTTP Client (Browser-Session)
```bash
onecrawl http get https://api.example.com/data
onecrawl http post https://api.example.com/submit --body '{"key":"value"}'
onecrawl http head https://example.com
onecrawl http fetch '{"url":"...","method":"POST","headers":{"Auth":"Bearer tok"}}'
```

## Node.js API
```javascript
// Data pipeline
const pipeline = {
  name: 'clean-data',
  steps: [
    { Filter: { field: 'price', operator: 'not_empty', value: '' } },
    { Transform: { field: 'name', transform: 'trim' } },
    { Deduplicate: { field: 'url' } }
  ]
};
const result = browser.pipelineExecute(JSON.stringify(pipeline), JSON.stringify(items));
browser.pipelineExport(result, 'output.csv', 'csv');

// HTTP client
const response = await browser.httpGet('https://api.example.com/data');
const json = await browser.httpFetchJson('https://api.example.com/data');
```

## Python API
```python
# Pipeline
pipeline = {
    "name": "clean-data",
    "steps": [
        {"Filter": {"field": "price", "operator": "not_empty", "value": ""}},
        {"Deduplicate": {"field": "url"}}
    ]
}
result = browser.pipeline_execute(json.dumps(pipeline), json.dumps(items))

# HTTP
response = browser.http_get('https://api.example.com/data')
```
