import { describe, expect, it } from 'vitest';
import { parseEngineEvent } from './engine-protocol';

const validResult = {
  event: 'result',
  status: 'success',
  data: {
    processed_records: 1999,
    engine_python_version: '3.11.9',
    dataset_sha256: 'a'.repeat(64),
    completed_at: '2026-11-07T10:00:00+00:00',
    loaded_file: '/opt/fairmorph/commands/poc.py'
  }
};

describe('Engine NDJSON protocol', () => {
  it('accepts a valid progress event', () => {
    const event = parseEngineEvent(JSON.stringify({
      event: 'progress',
      stage: 'PROCESSING',
      percentage: 50,
      message: 'Processing records'
    }));
    expect(event.event).toBe('progress');
  });

  it('accepts a complete success result', () => {
    const event = parseEngineEvent(JSON.stringify(validResult));
    expect(event.event).toBe('result');
    if (event.event === 'result' && event.status === 'success') {
      expect(event.data.processed_records).toBe(1999);
    }
  });

  it('accepts a failure result with a message', () => {
    expect(parseEngineEvent(JSON.stringify({
      event: 'result',
      status: 'error',
      message: 'Engine failed'
    }))).toMatchObject({ status: 'error', message: 'Engine failed' });
  });

  it('rejects missing result fields', () => {
    const incomplete = { ...validResult, data: { processed_records: 3 } };
    expect(() => parseEngineEvent(JSON.stringify(incomplete))).toThrow();
  });

  it.each([-1, 101, Number.NaN, Number.POSITIVE_INFINITY])(
    'rejects an invalid progress percentage: %s',
    (percentage) => {
      expect(() => parseEngineEvent(JSON.stringify({
        event: 'progress',
        stage: 'PROCESSING',
        percentage,
        message: 'Processing'
      }))).toThrow();
    }
  );

  it('rejects a result with a missing error message', () => {
    expect(() => parseEngineEvent(JSON.stringify({
      event: 'result',
      status: 'error'
    }))).toThrow();
  });

  it('rejects malformed NDJSON', () => {
    expect(() => parseEngineEvent('{not-json')).toThrow();
  });

  it('rejects unknown event types', () => {
    expect(() => parseEngineEvent(JSON.stringify({ event: 'unknown' }))).toThrow();
  });

  it('rejects invalid SHA-256 digests', () => {
    expect(() => parseEngineEvent(JSON.stringify({
      ...validResult,
      data: { ...validResult.data, dataset_sha256: 'not-a-digest' }
    }))).toThrow();
  });
});
