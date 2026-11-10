import { describe, expect, it } from 'vitest';
import { EngineEvent, HandshakeResult, ProgressEvent, RunResult, parseEngineEvent } from './engine-protocol';

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
