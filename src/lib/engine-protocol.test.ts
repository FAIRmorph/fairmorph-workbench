import { describe, it, expect } from 'vitest';
import { z } from 'zod';

const ResultPayloadSchema = z.object({
  event: z.literal('result'),
  status: z.literal('success'),
  data: z.object({
    processed_records: z.number().int().positive(),
    engine_python_version: z.string(),
    dataset_sha256: z.string().length(64),
    loaded_file: z.string()
  })
});

describe('Engine NDJSON Protocol', () => {
  it('validates a successful result payload schema', () => {
    const mockJson = {
      event: 'result',
      status: 'success',
      data: {
        processed_records: 1999,
        engine_python_version: '3.12.0',
        dataset_sha256: 'a'.repeat(64),
        loaded_file: 'File: /tmp/test.py'
      }
    };

    const parsed = ResultPayloadSchema.safeParse(mockJson);
    expect(parsed.success).toBe(true);
  });

  it('rejects incomplete payload structures', () => {
    const invalidJson = {
      event: 'result',
      status: 'error'
    };

    const parsed = ResultPayloadSchema.safeParse(invalidJson);
    expect(parsed.success).toBe(false);
  });
});