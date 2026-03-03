import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { NativeBrowser } from '../index.js';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

describe('Data Pipeline', () => {
  let b;
  before(async () => { b = await NativeBrowser.launch(true); });
  after(async () => { if (b) await b.close(); });

  const items = [
    { name: 'Alice', age: '30', city: 'Rome' },
    { name: 'Bob', age: '25', city: 'Milan' },
    { name: 'Charlie', age: '30', city: 'Rome' },
    { name: 'Alice', age: '35', city: 'Naples' },
    { name: 'Diana', age: '28', city: 'Turin' },
  ];

  it('execute pipeline with filter step', () => {
    const pipeline = JSON.stringify({
      name: 'filter-test',
      steps: [{ Filter: { field: 'city', operator: 'eq', value: 'Rome' } }],
    });
    const raw = b.pipelineExecute(pipeline, JSON.stringify(items));
    const result = JSON.parse(raw);
    assert.equal(result.input_count, 5);
    assert.equal(result.output_count, 2);
    assert.equal(result.filtered_count, 3);
    assert.ok(result.items.every(i => i.city === 'Rome'));
  });

  it('execute pipeline with transform step', () => {
    const pipeline = JSON.stringify({
      name: 'transform-test',
      steps: [{ Transform: { field: 'name', transform: 'uppercase' } }],
    });
    const raw = b.pipelineExecute(pipeline, JSON.stringify(items));
    const result = JSON.parse(raw);
    assert.equal(result.output_count, 5);
    assert.equal(result.items[0].name, 'ALICE');
    assert.equal(result.items[1].name, 'BOB');
  });

  it('execute pipeline with deduplicate step', () => {
    const pipeline = JSON.stringify({
      name: 'dedup-test',
      steps: [{ Deduplicate: { field: 'name' } }],
    });
    const raw = b.pipelineExecute(pipeline, JSON.stringify(items));
    const result = JSON.parse(raw);
    assert.equal(result.output_count, 4);
    assert.equal(result.deduplicated_count, 1);
  });

  it('execute pipeline with sort step', () => {
    const pipeline = JSON.stringify({
      name: 'sort-test',
      steps: [{ Sort: { field: 'age', descending: true } }],
    });
    const raw = b.pipelineExecute(pipeline, JSON.stringify(items));
    const result = JSON.parse(raw);
    assert.equal(result.items[0].age, '35');
    assert.equal(result.items[result.items.length - 1].age, '25');
  });

  it('validate pipeline returns errors for bad config', () => {
    const pipeline = JSON.stringify({
      name: '',
      steps: [{ Filter: { field: '', operator: 'invalid', value: '' } }],
    });
    const raw = b.pipelineValidate(pipeline);
    const errors = JSON.parse(raw);
    assert.ok(errors.length >= 2);
    assert.ok(errors.some(e => e.includes('pipeline name is empty')));
    assert.ok(errors.some(e => e.includes('filter field is empty')));
  });

  it('save and load pipeline roundtrip', () => {
    const tmp = path.join(os.tmpdir(), `pipeline-test-${Date.now()}.json`);
    const pipeline = JSON.stringify({
      name: 'roundtrip',
      steps: [{ Limit: { count: 10 } }],
    });
    b.pipelineSave(pipeline, tmp);
    const loaded = b.pipelineLoad(tmp);
    const parsed = JSON.parse(loaded);
    assert.equal(parsed.name, 'roundtrip');
    assert.equal(parsed.steps.length, 1);
    fs.unlinkSync(tmp);
  });

  it('export pipeline results as csv', () => {
    const pipeline = JSON.stringify({
      name: 'export-test',
      steps: [{ Limit: { count: 2 } }],
    });
    const raw = b.pipelineExecute(pipeline, JSON.stringify(items));
    const tmp = path.join(os.tmpdir(), `pipeline-export-${Date.now()}.csv`);
    const count = b.pipelineExport(raw, tmp, 'csv');
    assert.equal(count, 2);
    const csv = fs.readFileSync(tmp, 'utf8');
    assert.ok(csv.includes('name'));
    assert.ok(csv.includes('Alice'));
    fs.unlinkSync(tmp);
  });

  it('execute multi-step pipeline', () => {
    const pipeline = JSON.stringify({
      name: 'multi-step',
      steps: [
        { Filter: { field: 'city', operator: 'neq', value: 'Turin' } },
        { Deduplicate: { field: 'name' } },
        { Transform: { field: 'name', transform: 'lowercase' } },
        { Sort: { field: 'age', descending: false } },
        { Limit: { count: 2 } },
      ],
    });
    const raw = b.pipelineExecute(pipeline, JSON.stringify(items));
    const result = JSON.parse(raw);
    assert.equal(result.input_count, 5);
    assert.ok(result.output_count <= 2);
    assert.ok(result.items.every(i => i.name === i.name.toLowerCase()));
  });
});
