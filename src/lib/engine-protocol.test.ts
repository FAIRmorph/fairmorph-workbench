import { describe, expect, it } from 'vitest';
import { ApprovedDatasetMetadata, DatasetMetadataStatus, CreatedProject, ImportedResource, PipelineTemplate, SavedPipelineDraft, RegisteredPipeline, SelectedPipeline, ProjectVerification, EngineEvent, HandshakeResult, ProgressEvent, RunResult, StagedDatasetMetadata, parseEngineEvent } from './engine-protocol';

describe('Engine v0.13 RUN/2 boundary', () => {
  it('accepts Engine handshake and run support', () => {
    expect(HandshakeResult.parse({
      protocol_version: 'fairmorph.poc.ndjson/1',
      run_schema_version: 'fairmorph.poc.run/2',
      engine_version: '0.1.0',
      commands: ['run-pipeline']
    }).commands).toContain('run-pipeline');
  });

  it('rejects an incompatible run contract', () => {
    expect(() => HandshakeResult.parse({
      protocol_version: 'fairmorph.poc.ndjson/1',
      run_schema_version: 'fairmorph.poc.run/1',
      engine_version: '0.1.0',
      commands: ['run-pipeline']
    })).toThrow();
  });

  it('parses valid RUN output summaries', () => {
    const result = RunResult.parse({run_id: '20261110T120000Z-123456abcdef',
      outputs: [{name: 'samplingevent', rows: 2, columns: ['entity_id', 'event_type']}]});
    expect(result.outputs[0].rows).toBe(2);
  });

  it('rejects missing RUN result fields and invalid counts', () => {
    expect(() => RunResult.parse({run_id: 'foo'})).toThrow();
    expect(() => RunResult.parse({run_id: 'foo',
      outputs: [{name: 'events', rows: -1, columns: []}]})).toThrow();
  });

  it('parses NDJSON progress', () => {
    expect(parseEngineEvent(JSON.stringify({
      event: 'progress', stage: 'PROCESSING', percentage: 50, message: 'Running'
    })).event).toBe('progress');
  });

  it('rejects impossible percentages and unexpected events', () => {
    expect(ProgressEvent.safeParse({
      event: 'progress', stage: 'RUN', percentage: 101, message: 'Error'
    }).success).toBe(false);
    expect(EngineEvent.safeParse({event: 'benchmark'}).success).toBe(false);
  });

  it('rejects malformed output and accepts an Engine error envelope', () => {
    expect(() => parseEngineEvent('{invalid')).toThrow();
    expect(parseEngineEvent({event: 'result', status: 'error', message: 'Unapproved pipeline'}))
      .toMatchObject({status: 'error'});
  });
});

describe('Project-file dataset metadata revision responses', () => {
  const sha = 'a'.repeat(64);
  it('accepts unconfigured and explicitly approved metadata', () => {
    expect(DatasetMetadataStatus.parse({
      status: 'unconfigured', title: null, revision_sha256: null
    }).status).toBe('unconfigured');
    expect(DatasetMetadataStatus.parse({
      status: 'approved', title: 'Confirmed scientific title',
      revision_sha256: sha, confirmed_by: 'curator', confirmed_at: '2026-11-10T12:00:00Z'
    }).status).toBe('approved');
  });
  it('rejects malformed staged SHA and missing approving user', () => {
    expect(StagedDatasetMetadata.safeParse({
      status: 'staged', revision_sha256: 'bad', title: 'A title'
    }).success).toBe(false);
    expect(ApprovedDatasetMetadata.safeParse({
      status: 'approved', revision_sha256: sha, title: 'A title',
      confirmed_by: '', confirmed_at: '2026-11-10'
    }).success).toBe(false);
  });
});

describe('Project setup Engine result contracts', () => {
  it('accepts Engine-generated project identity', () => {
    expect(CreatedProject.parse({
      project_id: '12345678-1234-4234-8234-123456789abc',
      project_path: '/tmp/my-project.fairmorph'
    }).project_path).toBe('/tmp/my-project.fairmorph');
  });
  it('requires a recognized role and valid source checksum', () => {
    const source = {
      resource_id: 'resource-events-abc', role: 'events',
      original_name: 'events.csv', sha256: 'a'.repeat(64)
    };
    expect(ImportedResource.parse(source).role).toBe('events');
    expect(ImportedResource.safeParse({ ...source, role: 'unknown' }).success).toBe(false);
    expect(ImportedResource.safeParse({ ...source, sha256: 'broken' }).success).toBe(false);
  });
});

describe('technical pipeline setup protocol', () => {
  const sha = 'b'.repeat(64);
  const project_id = '12345678-1234-4234-8234-123456789abc';

  it('accepts explicit technical input slots, pipeline outputs and step count', () => {
    expect(PipelineTemplate.parse({
      path: '/tmp/template.json',
      inputs: [{name: 'survey', format: 'csv'}, {name: 'counts', format: 'parquet'}],
      outputs: ['normalized'], steps: 2
    }).inputs).toHaveLength(2);
  });
  it('rejects missing inputs, malformed formats and unpopulated templates', () => {
    const template = {
      path: '/tmp/template.json',
      inputs: [{name: 'events', format: 'csv'}], outputs: ['out'], steps: 1
    };
    expect(PipelineTemplate.safeParse({...template, inputs: []}).success).toBe(false);
    expect(PipelineTemplate.safeParse({...template, inputs: [{name: 'events', format: 'json'}]}).success).toBe(false);
    expect(PipelineTemplate.safeParse({...template, steps: 0}).success).toBe(false);
  });
  it('requires real Engine registration and selection identities', () => {
    expect(SavedPipelineDraft.parse({path: '/tmp/project/configurations/drafts/pipeline-technical.json', inputs: 2}).inputs).toBe(2);
    expect(RegisteredPipeline.parse({pipeline_sha256: sha}).pipeline_sha256).toBe(sha);
    expect(SelectedPipeline.parse({project_id, active_pipeline_sha256: sha}).active_pipeline_sha256).toBe(sha);
    expect(RegisteredPipeline.safeParse({pipeline_sha256: '123'}).success).toBe(false);
    expect(SelectedPipeline.safeParse({project_id, active_pipeline_sha256: 'not-a-sha'}).success).toBe(false);
  });
});

describe('Engine project integrity verification contract', () => {
  const valid = {
    project_path: '/tmp/project.fairmorph',
    source_objects: 2,
    completed_runs: 1,
    exports: 0,
    status: 'verified'
  };

  it('accepts the Engine verify-project result and zero exports', () => {
    expect(ProjectVerification.parse(valid)).toEqual(valid);
    expect(ProjectVerification.parse({ ...valid, completed_runs: 0 }).completed_runs).toBe(0);
  });

  it('rejects failed status and malformed or negative counters', () => {
    expect(ProjectVerification.safeParse({ ...valid, status: 'invalid' }).success).toBe(false);
    expect(ProjectVerification.safeParse({ ...valid, source_objects: -1 }).success).toBe(false);
    expect(ProjectVerification.safeParse({ ...valid, completed_runs: 1.5 }).success).toBe(false);
    expect(ProjectVerification.safeParse({ ...valid, exports: '0' }).success).toBe(false);
    expect(ProjectVerification.safeParse({ ...valid, project_path: '' }).success).toBe(false);
  });
});
