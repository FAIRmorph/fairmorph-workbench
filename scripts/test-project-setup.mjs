/**
 * Data-independent Engine CLI integration test for the Workbench setup sequence.
 * Uses synthetic inputs only. Never executes a scientific RUN.
 * Run with: node scripts/test-project-setup.mjs (Engine installed in Python).
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const python = process.env.FAIRMORPH_ENGINE_PYTHON || (process.platform === 'win32' ? 'python' : 'python3');
const root = mkdtempSync(join(tmpdir(), 'fairmorph-setup-integration-'));
const project = join(root, 'synthetic.fairmorph');
const sources = join(root, 'sources');
mkdirSync(sources);
const events = join(sources, 'events.csv');
const counts = join(sources, 'counts.csv');
writeFileSync(events, 'id,date\n1,2026-01-01\n2,2026-01-02\n');
writeFileSync(counts, 'id,count\n1,0\n2,4\n');
const sourceBytes = new Map([[events, readFileSync(events)], [counts, readFileSync(counts)]]);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');

function cli(command, args = [], success = true) {
  const result = spawnSync(python, ['-m', 'fairmorph_engine.cli', command, ...args], {
    encoding: 'utf8', timeout: 60_000
  });
  if (result.error) throw result.error;
  const events = (result.stdout || '').trim().split(/\r?\n/).filter(Boolean)
    .map(line => JSON.parse(line));
  const results = events.filter(event => event.event === 'result');
  assert.equal(results.length, 1, `Exactly one result for ${command}: ${result.stdout}; ${result.stderr}`);
  const last = results[0];
  assert.equal(last.status, success ? 'success' : 'error',
    `${command}: ${result.stderr || last.message}`);
  assert.equal(result.status === 0, success, `Process exit code for ${command}`);
  return success ? last.data : last.message;
}

try {
  const created = cli('init-project', ['--project', project, '--name', 'Synthetic Test']);
  assert.equal(created.project_path, project);
  assert.match(created.project_id, /^[0-9a-f-]{36}$/);
  assert.equal(JSON.parse(readFileSync(join(project, 'project.json')))['name'], 'Synthetic Test');
  assert.match(cli('run-pipeline', ['--project', project], false), /mapping|pipeline/i);

  const imported = {};
  for (const [role, source] of [['events', events], ['counts', counts]]) {
    const result = cli('import-resource', [
      '--project', project, '--source', source, '--role', role
    ]);
    imported[role] = result;
    assert.equal(result.role, role);
    assert.equal(result.original_name, source.split(/[\\/]/).at(-1));
    assert.equal(result.sha256, hash(sourceBytes.get(source)));
    assert.ok(result.resource_id.startsWith('resource-' + role + '-'));
    assert.deepEqual(readFileSync(join(project, result.object_path)), sourceBytes.get(source));
    assert.deepEqual(readFileSync(source), sourceBytes.get(source), 'Input file must be untouched');
  }

  // Reimporting identical original content must not create a duplicate immutable object.
  const duplicate = cli('import-resource', [
    '--project', project, '--source', events, '--role', 'events'
  ]);
  assert.equal(duplicate.resource_id, imported.events.resource_id);
  const stored = readdirSync(join(project, 'objects', 'sha256'), { recursive: true })
    .filter(name => /^[a-f0-9]{64}$/.test(String(name).split('/').at(-1)));
  assert.equal(stored.length, 2);

  // The Workbench only fills the explicitly selected source bindings.
  // No scientific metadata/field mapping is injected.
  const originalSteps = [{
    id: 'select_events', operator: 'table.select/1',
    inputs: ['events'], output: 'selected_events',
    parameters: { columns: ['id', 'date'] }
  }];
  const draft = {
    schema_version: 'fairmorph.pipeline/1',
    resources: {
      events: { resource_id: imported.events.resource_id, format: 'csv' },
      counts: { resource_id: imported.counts.resource_id, format: 'csv' }
    },
    steps: originalSteps,
    outputs: ['selected_events']
  };
  const draftPath = join(project, 'configurations', 'drafts', 'pipeline-technical.json');
  mkdirSync(join(project, 'configurations', 'drafts'), { recursive: true });
  writeFileSync(draftPath, JSON.stringify(draft, null, 2) + '\n');
  const registered = cli('register-pipeline', [
    '--project', project, '--definition', draftPath
  ]);
  assert.match(registered.pipeline_sha256, /^[0-9a-f]{64}$/);
  const pinned = JSON.parse(readFileSync(join(
    project, 'configurations', 'pipelines', registered.pipeline_sha256 + '.json'
  )));
  assert.deepEqual(pinned.steps, originalSteps);
  assert.deepEqual(pinned.resources, draft.resources);
  assert.match(cli('select-pipeline', [
    '--project', project, '--pipeline-sha256', '0'.repeat(64)
  ], false), /missing|pipeline|config/i);
  const selected = cli('select-pipeline', [
    '--project', project, '--pipeline-sha256', registered.pipeline_sha256
  ]);
  assert.equal(selected.active_pipeline_sha256, registered.pipeline_sha256);
  const descriptor = JSON.parse(readFileSync(join(project, 'project.json')));
  assert.equal(descriptor.active_pipeline_sha256, registered.pipeline_sha256);
  assert.deepEqual(descriptor.run_ids, []);
  assert.deepEqual(readdirSync(join(project, 'runs')), []);
  assert.deepEqual(readFileSync(events), sourceBytes.get(events));
  assert.deepEqual(readFileSync(counts), sourceBytes.get(counts));
  const beforeVerify = readFileSync(join(project, 'project.json'));
  const verification = cli('verify-project', ['--project', project]);
  assert.equal(verification.status, 'verified');
  assert.equal(verification.source_objects, 2);
  assert.equal(verification.completed_runs, 0);
  assert.equal(verification.exports, 0);
  assert.deepEqual(readFileSync(join(project, 'project.json')), beforeVerify,
    'verify-project must not change the project descriptor');
  assert.deepEqual(readdirSync(join(project, 'runs')), [],
    'verify-project must not trigger a RUN');
  console.log('Project setup integration PASS: init/import/dedupe/register/select/verify; no RUN or source modification');
} finally {
  rmSync(root, { recursive: true, force: true });
}
